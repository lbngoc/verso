//! Self-update (FR-015/016/017, contracts/update-channel.md).
//!
//! The policy lives here as pure functions so it can be tested without a server: when a check is
//! allowed (never before onboarding ends, never when the user turned it off), when a release is an
//! offer (newer than the running version, never a pre-release), and the schedule (30 s after start,
//! then every 4 hours of wall-clock time, spec 004 FR-029).
//!
//! The network side is `tauri-plugin-updater`; the private key never reaches the app, and the
//! signature is verified against the built-in public key before anything is installed.

use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

use parking_lot::Mutex;
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::Update;

use crate::commands::AppState;

/// First check 30 s after start (contracts/update-channel.md), then every 4 hours of wall-clock time
/// (spec 004 FR-029; was once a day).
pub const FIRST_CHECK_DELAY: Duration = Duration::from_secs(30);
pub const CHECK_INTERVAL: Duration = Duration::from_secs(4 * 60 * 60);
/// A failed check (offline, server busy) is tried again sooner.
pub const RETRY_AFTER_FAILURE: Duration = Duration::from_secs(30 * 60);
/// How often the loop looks at the clock: a check due after a sleep follows within this much.
pub const SCHEDULE_TICK: Duration = Duration::from_secs(15 * 60);
/// Spread of the interval, so installs do not all ask at the same minute (±10 % of 4 h).
pub const JITTER_MAX_SECS: i64 = 24 * 60;

/// When the next automatic check is due, measured on the wall clock (`SystemTime`), not on a
/// monotonic timer: on macOS a monotonic sleep stops while the machine sleeps, so the old daily
/// timer could slip by days. Pure, so it is tested without waiting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CheckSchedule {
    last_success: Option<SystemTime>,
    last_failure: Option<SystemTime>,
    /// Added to the interval after each success, in seconds (−JITTER_MAX_SECS..=JITTER_MAX_SECS).
    jitter_secs: i64,
}

impl CheckSchedule {
    pub const fn new() -> Self {
        Self {
            last_success: None,
            last_failure: None,
            jitter_secs: 0,
        }
    }

    /// Whether a check should run at `now`.
    pub fn due(&self, now: SystemTime) -> bool {
        let elapsed = |since: SystemTime| now.duration_since(since).ok();
        let failure_is_newer = match (self.last_failure, self.last_success) {
            (Some(failure), Some(success)) => failure > success,
            (Some(_), None) => true,
            _ => false,
        };
        if failure_is_newer {
            // A clock set back makes `elapsed` fail: check rather than wait for an unknown time.
            return self
                .last_failure
                .and_then(elapsed)
                .map_or(true, |waited| waited >= RETRY_AFTER_FAILURE);
        }
        match self.last_success {
            None => true,
            Some(success) => elapsed(success).map_or(true, |waited| {
                let interval = CHECK_INTERVAL.as_secs() as i64 + self.jitter_secs;
                waited.as_secs() as i64 >= interval
            }),
        }
    }

    /// A check finished (an offer or "up to date"); `jitter_secs` spreads the next interval.
    pub fn succeeded(&mut self, now: SystemTime, jitter_secs: i64) {
        self.last_success = Some(now);
        self.last_failure = None;
        self.jitter_secs = jitter_secs.clamp(-JITTER_MAX_SECS, JITTER_MAX_SECS);
    }

    /// A check failed; the next one is due after `RETRY_AFTER_FAILURE`.
    pub fn failed(&mut self, now: SystemTime) {
        self.last_failure = Some(now);
    }
}

/// The offer announced to the UI and kept for the Install button (spec 009, data-model `UpdateOffer`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateInfo {
    pub version: String,
    /// The running version, so the dialog can say "you have X".
    pub current: String,
    pub notes: String,
    /// The user skipped this exact version earlier (manual checks still show it).
    pub skipped: bool,
}

/// Where the update flow stands (spec 011 data-model § Update status). Held here, in one place, so
/// Settings › About, its sidebar badge and the tray item always agree, and a download keeps going when
/// Settings closes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    #[default]
    Idle,
    Checking,
    UpToDate,
    CheckFailed,
    Available,
    Downloading,
    ReadyOnQuit,
    Installing,
    InstallFailed,
}

impl Phase {
    /// A download or install is under way: a check must not replace it.
    pub fn busy(self) -> bool {
        matches!(
            self,
            Phase::Downloading | Phase::ReadyOnQuit | Phase::Installing
        )
    }
}

/// Why a check or an install failed, for a plain message in About.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    Offline,
    Download,
    Verify,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct UpdateStatus {
    pub phase: Phase,
    pub offer: Option<UpdateInfo>,
    /// Last successful check, RFC 3339 (UTC); persisted as `last_update_check_at`.
    pub last_checked: Option<String>,
    /// Only while downloading.
    pub progress: Option<Progress>,
    pub error: Option<FailureKind>,
}

/// What happened, fed to [`apply`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateEvent {
    CheckStarted,
    CheckFound {
        offer: UpdateInfo,
        at: String,
    },
    CheckNone {
        at: String,
    },
    CheckFailed,
    DownloadStarted,
    DownloadProgress(Progress),
    /// Install on Quit: the verified bytes are held.
    Downloaded,
    InstallStarted,
    DownloadFailed,
    InstallFailed,
    Skipped,
}

/// The transitions of data-model § Update status, pure so every row is tested. `manual` is true for
/// "Check for Updates…" and false for the automatic schedule.
pub fn apply(status: &UpdateStatus, event: UpdateEvent, manual: bool) -> UpdateStatus {
    let mut next = status.clone();
    let busy = status.phase.busy();
    match event {
        UpdateEvent::CheckStarted => {
            if manual && !busy {
                next.phase = Phase::Checking;
                next.error = None;
            }
        }
        UpdateEvent::CheckFound { offer, at } => {
            next.last_checked = Some(at);
            if !busy {
                next.phase = Phase::Available;
                next.offer = Some(offer);
                next.error = None;
                next.progress = None;
            }
        }
        UpdateEvent::CheckNone { at } => {
            next.last_checked = Some(at);
            if !busy {
                next.phase = if manual { Phase::UpToDate } else { Phase::Idle };
                next.offer = None;
                next.error = None;
                next.progress = None;
            }
        }
        UpdateEvent::CheckFailed => {
            if manual && !busy {
                next.phase = Phase::CheckFailed;
                next.error = Some(FailureKind::Offline);
            }
        }
        UpdateEvent::DownloadStarted => {
            if status.offer.is_some() && !busy {
                next.phase = Phase::Downloading;
                next.progress = None;
                next.error = None;
            }
        }
        UpdateEvent::DownloadProgress(progress) => {
            if status.phase == Phase::Downloading {
                next.progress = Some(progress);
            }
        }
        UpdateEvent::Downloaded => {
            if status.phase == Phase::Downloading {
                next.phase = Phase::ReadyOnQuit;
                next.progress = None;
            }
        }
        UpdateEvent::InstallStarted => {
            if status.offer.is_some() {
                next.phase = Phase::Installing;
                next.progress = None;
                next.error = None;
            }
        }
        UpdateEvent::DownloadFailed => {
            next.phase = Phase::InstallFailed;
            next.progress = None;
            next.error = Some(FailureKind::Download);
        }
        UpdateEvent::InstallFailed => {
            next.phase = Phase::InstallFailed;
            next.progress = None;
            next.error = Some(FailureKind::Verify);
        }
        UpdateEvent::Skipped => {
            next.phase = Phase::Idle;
            next.offer = None;
            next.progress = None;
            next.error = None;
        }
    }
    next
}

/// The badge on About and the tray item: an offer the user has not skipped (FR-013).
pub fn shows_badge(status: &UpdateStatus) -> bool {
    status.offer.as_ref().is_some_and(|offer| !offer.skipped)
}

static PENDING: OnceLock<Mutex<Option<Update>>> = OnceLock::new();
static PENDING_BYTES: OnceLock<Mutex<Option<Vec<u8>>>> = OnceLock::new();

fn pending() -> &'static Mutex<Option<Update>> {
    PENDING.get_or_init(|| Mutex::new(None))
}

fn pending_bytes() -> &'static Mutex<Option<Vec<u8>>> {
    PENDING_BYTES.get_or_init(|| Mutex::new(None))
}

/// `1.2.3` → `(1, 2, 3)`; anything else has no comparable core and is never offered.
fn core(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.trim().trim_start_matches('v');
    let core = core.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// A pre-release (`0.2.0-rc.1`) is never offered by the automatic channel.
pub fn is_prerelease(version: &str) -> bool {
    version.contains('-')
}

/// Only a strictly newer release is an update.
pub fn is_newer(installed: &str, candidate: &str) -> bool {
    match (core(installed), core(candidate)) {
        (Some(installed), Some(candidate)) => candidate > installed,
        _ => false,
    }
}

/// Whether a release announced by the endpoint should be offered.
pub fn should_offer(installed: &str, candidate: &str) -> bool {
    !is_prerelease(candidate) && is_newer(installed, candidate)
}

/// The automatic channel stays quiet about a version the user skipped; a manual check does not.
pub fn should_offer_auto(installed: &str, candidate: &str, skipped: Option<&str>) -> bool {
    should_offer(installed, candidate) && skipped != Some(candidate)
}

/// Automatic checks need a finished onboarding (the choice) and the user's consent.
pub fn should_check(onboarding_complete: bool, check_updates: bool) -> bool {
    onboarding_complete && check_updates
}

fn auto_check_enabled(app: &AppHandle) -> bool {
    let profile = app.state::<AppState>().settings.get();
    should_check(
        profile.first_run_engine_choice_at.is_some(),
        profile.check_updates,
    )
}

const CHECK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

static STATUS: OnceLock<Mutex<UpdateStatus>> = OnceLock::new();

fn status_cell() -> &'static Mutex<UpdateStatus> {
    STATUS.get_or_init(|| Mutex::new(UpdateStatus::default()))
}

/// Applies `event`, stores the result, tells every window (`update-status`) and keeps the tray item
/// in step with the badge.
fn transition(app: &AppHandle, event: UpdateEvent, manual: bool) -> UpdateStatus {
    let next = {
        let mut status = status_cell().lock();
        let next = apply(&status, event, manual);
        if next == *status {
            return next;
        }
        *status = next.clone();
        next
    };
    let _ = app.emit("update-status", &next);
    let badge = shows_badge(&next).then(|| next.offer.as_ref().map(|offer| offer.version.as_str()));
    crate::tray::set_update_available(app, badge.flatten());
    next
}

/// `secs` since the Unix epoch as RFC 3339 UTC (`2026-11-25T08:00:00Z`); no date crate for one
/// timestamp (civil-from-days, H. Hinnant).
pub fn rfc3339_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

fn now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    rfc3339_utc(secs)
}

/// Asks the endpoint. `Ok(Some)` is an offer to show, `Ok(None)` means nothing new for this channel.
async fn check(app: &AppHandle, manual: bool) -> Result<Option<(UpdateInfo, Update)>, String> {
    use tauri_plugin_updater::UpdaterExt;

    // Without a timeout an offline Mac waits for the OS connect timeout (~75 s) while About shows
    // "Checking…"; 15 s is plenty for a small JSON file.
    let updater = app
        .updater_builder()
        .timeout(CHECK_TIMEOUT)
        .build()
        .map_err(|error| error.to_string())?;
    let announced = updater.check().await.map_err(|error| error.to_string())?;
    let Some(update) = announced else {
        return Ok(None);
    };
    let skipped_version = app
        .state::<AppState>()
        .settings
        .get()
        .skipped_update_version;
    let offered = if manual {
        should_offer(env!("CARGO_PKG_VERSION"), &update.version)
    } else {
        should_offer_auto(
            env!("CARGO_PKG_VERSION"),
            &update.version,
            skipped_version.as_deref(),
        )
    };
    if !offered {
        log::info!(
            "update endpoint announced {}, which is not offered",
            update.version
        );
        return Ok(None);
    }
    let info = UpdateInfo {
        version: update.version.clone(),
        current: env!("CARGO_PKG_VERSION").to_string(),
        notes: update.body.clone().unwrap_or_default(),
        skipped: skipped_version.as_deref() == Some(update.version.as_str()),
    };
    log::info!("update available: {}", info.version);
    Ok(Some((info, update)))
}

/// One check, automatic or manual, run through the state machine. The endpoint's answer replaces
/// the held `Update` only while no download or install is under way.
async fn run_check(app: &AppHandle, manual: bool) -> Result<UpdateStatus, String> {
    transition(app, UpdateEvent::CheckStarted, manual);
    match check(app, manual).await {
        Ok(found) => {
            let at = now_rfc3339();
            let state = app.state::<AppState>();
            if let Err(error) = state
                .settings
                .update(&json!({ "last_update_check_at": at }))
            {
                log::warn!("could not save the update check time: {error}");
            }
            let busy = status_cell().lock().phase.busy();
            let event = match found {
                Some((offer, update)) => {
                    if !busy {
                        *pending().lock() = Some(update);
                        *pending_bytes().lock() = None;
                    }
                    UpdateEvent::CheckFound { offer, at }
                }
                None => {
                    if !busy {
                        *pending().lock() = None;
                        *pending_bytes().lock() = None;
                    }
                    UpdateEvent::CheckNone { at }
                }
            };
            Ok(transition(app, event, manual))
        }
        Err(error) => {
            transition(app, UpdateEvent::CheckFailed, manual);
            Err(error)
        }
    }
}

/// Starts the automatic schedule. Nothing is requested before the onboarding choice is saved; then
/// the first check follows about 30 s later and the next ones come every 4 hours of wall-clock time
/// (a failed one again after 30 minutes). An offer updates the tray item and the badge on About.
pub fn start(app: AppHandle) {
    let last_checked = app.state::<AppState>().settings.get().last_update_check_at;
    status_cell().lock().last_checked = last_checked;
    tauri::async_runtime::spawn(async move {
        let mut schedule = CheckSchedule::new();
        loop {
            let onboarded = app
                .state::<AppState>()
                .settings
                .get()
                .first_run_engine_choice_at
                .is_some();
            // Before the first check (and until onboarding ends) look every 30 s; afterwards the
            // clock is read every 15 minutes.
            let wait = if onboarded && schedule != CheckSchedule::new() {
                SCHEDULE_TICK
            } else {
                FIRST_CHECK_DELAY
            };
            tokio::time::sleep(wait).await;

            if !onboarded || !auto_check_enabled(&app) || !schedule.due(SystemTime::now()) {
                continue;
            }
            match run_check(&app, false).await {
                Ok(_) => {
                    let jitter = rand::random_range(-JITTER_MAX_SECS..=JITTER_MAX_SECS);
                    schedule.succeeded(SystemTime::now(), jitter);
                }
                Err(error) => {
                    schedule.failed(SystemTime::now());
                    log::warn!("update check failed: {error}; trying again in 30 minutes");
                }
            }
        }
    });
}

/// What About shows when it opens: from memory, no network (SC-003).
#[tauri::command]
pub fn update_status() -> UpdateStatus {
    status_cell().lock().clone()
}

/// "Check for Updates…": always requests, whatever the preference says; the result is shown in
/// place in About (a failure is reported through the status, not as an error).
#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> UpdateStatus {
    match run_check(&app, true).await {
        Ok(status) => status,
        Err(error) => {
            log::warn!("manual update check failed: {error}");
            update_status()
        }
    }
}

/// Download progress shown in About's update card (data-model `UpdateProgress`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Progress {
    pub downloaded: u64,
    /// `None` when the server did not say how long the file is: the bar is indeterminate.
    pub total: Option<u64>,
    pub percent: Option<u32>,
}

/// Bytes between two reports when the length is unknown.
const UNKNOWN_LENGTH_STEP: u64 = 256 * 1024;

/// Turns the updater's per-chunk callback into progress. `tauri-plugin-updater` calls
/// `on_chunk(chunk.len(), content_length)`: the first argument is the size of *this chunk*, not a
/// running total; treating it as the total kept the bar near 0 % in 0.1.7 (spec 004 R11).
#[derive(Debug, Default)]
pub struct ProgressTracker {
    downloaded: u64,
    last_percent: Option<u32>,
    last_report: Option<u64>,
}

impl ProgressTracker {
    pub fn on_chunk(&mut self, chunk_len: usize, total: Option<u64>) -> Option<Progress> {
        self.downloaded += chunk_len as u64;
        match total.filter(|total| *total > 0) {
            Some(total) => {
                let percent = ((self.downloaded.min(total)) * 100 / total) as u32;
                // Monotonic, and one report per percent step.
                if self.last_percent.is_some_and(|last| percent <= last) {
                    return None;
                }
                self.last_percent = Some(percent);
                Some(Progress {
                    downloaded: self.downloaded,
                    total: Some(total),
                    percent: Some(percent),
                })
            }
            None => {
                let due = self
                    .last_report
                    .map_or(true, |last| self.downloaded - last >= UNKNOWN_LENGTH_STEP);
                if !due {
                    return None;
                }
                self.last_report = Some(self.downloaded);
                Some(Progress {
                    downloaded: self.downloaded,
                    total: None,
                    percent: None,
                })
            }
        }
    }
}

fn progress_reporter(app: &AppHandle) -> impl FnMut(usize, Option<u64>) + 'static {
    let app = app.clone();
    let mut tracker = ProgressTracker::default();
    move |chunk_len, total| {
        if let Some(progress) = tracker.on_chunk(chunk_len, total) {
            transition(&app, UpdateEvent::DownloadProgress(progress), true);
        }
    }
}

/// Downloads, verifies and applies the offered release, then relaunches. With bytes already held by
/// "Install on Quit" it installs them at once.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let update = pending()
        .lock()
        .clone()
        .ok_or_else(|| "no update is available".to_string())?;
    let held = pending_bytes().lock().take();
    let result = match held {
        Some(bytes) => {
            transition(&app, UpdateEvent::InstallStarted, true);
            update.install(bytes).map_err(|error| (true, error))
        }
        None => {
            transition(&app, UpdateEvent::DownloadStarted, true);
            let finished = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let on_finish = {
                let app = app.clone();
                let finished = finished.clone();
                move || {
                    finished.store(true, std::sync::atomic::Ordering::SeqCst);
                    transition(&app, UpdateEvent::InstallStarted, true);
                }
            };
            update
                .download_and_install(progress_reporter(&app), on_finish)
                .await
                .map_err(|error| (finished.load(std::sync::atomic::Ordering::SeqCst), error))
        }
    };
    if let Err((downloaded, error)) = result {
        // About shows a plain message by kind; the detail belongs in the log.
        log::warn!("update could not be installed: {error}");
        let event = if downloaded {
            UpdateEvent::InstallFailed
        } else {
            UpdateEvent::DownloadFailed
        };
        transition(&app, event, true);
        return Err(error.to_string());
    }

    log::info!("update installed; relaunching");
    // Windows: the installer exits the app itself, so this only runs on macOS/Linux.
    #[cfg(not(windows))]
    app.restart();

    #[cfg(windows)]
    Ok(())
}

/// "Install on Quit": download and verify now, keep the bytes, install when the app exits.
#[tauri::command]
pub async fn install_update_on_quit(app: AppHandle) -> Result<(), String> {
    let update = pending()
        .lock()
        .clone()
        .ok_or_else(|| "no update is available".to_string())?;
    transition(&app, UpdateEvent::DownloadStarted, true);
    match update.download(progress_reporter(&app), || {}).await {
        Ok(bytes) => {
            *pending_bytes().lock() = Some(bytes);
            transition(&app, UpdateEvent::Downloaded, true);
            log::info!("update verified; it will be installed when Verso quits");
            Ok(())
        }
        Err(error) => {
            log::warn!("update could not be downloaded: {error}");
            transition(&app, UpdateEvent::DownloadFailed, true);
            Err(error.to_string())
        }
    }
}

/// Runs from `RunEvent::ExitRequested`: installs the verified bytes, if any.
pub fn install_pending_on_exit(_app: &AppHandle) {
    let bytes = pending_bytes().lock().take();
    let Some(bytes) = bytes else {
        return;
    };
    let Some(update) = pending().lock().clone() else {
        return;
    };
    match update.install(bytes) {
        Ok(()) => log::info!("pending update installed on quit"),
        Err(error) => log::warn!("pending update could not be installed: {error}"),
    }
}

/// "Skip This Version": remember it, forget the offer, clear the tray item and the badge.
#[tauri::command]
pub fn skip_update_version(app: AppHandle, version: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let profile = state
        .settings
        .update(&json!({ "skipped_update_version": version }))
        .map_err(|error| error.to_string())?;
    *pending().lock() = None;
    *pending_bytes().lock() = None;
    transition(&app, UpdateEvent::Skipped, true);
    let _ = app.emit("settings-changed", &profile);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_newer_version_is_an_update() {
        assert!(is_newer("0.1.0", "0.1.1"));
        assert!(is_newer("0.1.9", "0.2.0"));
        assert!(is_newer("1.0.0", "2.0.0"));
        assert!(!is_newer("0.1.1", "0.1.1"));
        assert!(!is_newer("0.1.2", "0.1.1"));
        assert!(!is_newer("0.1.0", "not-a-version"));
    }

    #[test]
    fn pre_releases_are_never_offered() {
        assert!(should_offer("0.1.0", "0.2.0"));
        assert!(!should_offer("0.1.0", "0.2.0-rc.1"));
        assert!(!should_offer("0.1.0", "0.1.0"));
        assert!(is_prerelease("0.1.0-rc.2"));
        assert!(!is_prerelease("0.1.0"));
    }

    #[test]
    fn no_check_before_onboarding_or_without_consent() {
        assert!(!should_check(false, true));
        assert!(!should_check(false, false));
        assert!(!should_check(true, false));
        assert!(should_check(true, true));
    }

    #[test]
    fn a_skipped_version_is_hidden_from_the_automatic_channel_only() {
        assert!(should_offer_auto("0.1.0", "0.2.0", None));
        assert!(!should_offer_auto("0.1.0", "0.2.0", Some("0.2.0")));
        // A newer version than the skipped one is offered again.
        assert!(should_offer_auto("0.1.0", "0.3.0", Some("0.2.0")));
        // Manual checks still see the skipped version.
        assert!(should_offer("0.1.0", "0.2.0"));
    }

    fn at(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000 + seconds)
    }

    #[test]
    fn the_first_check_is_due_at_once_then_every_four_hours() {
        let mut schedule = CheckSchedule::new();
        assert!(schedule.due(at(0)));
        schedule.succeeded(at(0), 0);
        assert!(!schedule.due(at(4 * 3600 - 1)));
        assert!(schedule.due(at(4 * 3600)));
    }

    #[test]
    fn a_check_is_due_right_after_a_long_sleep() {
        // The wall clock moved 9 hours while the machine slept: due at the next tick.
        let mut schedule = CheckSchedule::new();
        schedule.succeeded(at(0), 0);
        assert!(schedule.due(at(9 * 3600)));
    }

    #[test]
    fn a_failed_check_is_retried_after_thirty_minutes() {
        let mut schedule = CheckSchedule::new();
        schedule.succeeded(at(0), 0);
        schedule.failed(at(4 * 3600));
        assert!(!schedule.due(at(4 * 3600 + 29 * 60)));
        assert!(schedule.due(at(4 * 3600 + 30 * 60)));
        schedule.succeeded(at(4 * 3600 + 30 * 60), 0);
        assert!(!schedule.due(at(5 * 3600)));
    }

    #[test]
    fn the_jitter_moves_the_interval_within_ten_percent() {
        let mut schedule = CheckSchedule::new();
        schedule.succeeded(at(0), 10_000);
        // Clamped to +24 minutes.
        assert!(!schedule.due(at(4 * 3600 + 24 * 60 - 1)));
        assert!(schedule.due(at(4 * 3600 + 24 * 60)));
        schedule.succeeded(at(0), -24 * 60);
        assert!(schedule.due(at(4 * 3600 - 24 * 60)));
    }

    #[test]
    fn a_clock_set_back_makes_the_check_due() {
        let mut schedule = CheckSchedule::new();
        schedule.succeeded(at(10_000), 0);
        assert!(schedule.due(at(0)));
    }

    fn offer(version: &str, skipped: bool) -> UpdateInfo {
        UpdateInfo {
            version: version.into(),
            current: "0.1.11".into(),
            notes: "- Faster".into(),
            skipped,
        }
    }

    fn in_phase(phase: Phase, offer_set: bool) -> UpdateStatus {
        UpdateStatus {
            phase,
            offer: offer_set.then(|| offer("0.1.12", false)),
            last_checked: Some("2026-11-25T08:00:00Z".into()),
            ..UpdateStatus::default()
        }
    }

    fn found(version: &str) -> UpdateEvent {
        UpdateEvent::CheckFound {
            offer: offer(version, false),
            at: "2026-11-25T12:00:00Z".into(),
        }
    }

    fn none() -> UpdateEvent {
        UpdateEvent::CheckNone {
            at: "2026-11-25T12:00:00Z".into(),
        }
    }

    #[test]
    fn only_a_manual_check_shows_checking() {
        let idle = UpdateStatus::default();
        assert_eq!(
            apply(&idle, UpdateEvent::CheckStarted, true).phase,
            Phase::Checking
        );
        assert_eq!(apply(&idle, UpdateEvent::CheckStarted, false), idle);
        let up_to_date = in_phase(Phase::UpToDate, false);
        assert_eq!(
            apply(&up_to_date, UpdateEvent::CheckStarted, true).phase,
            Phase::Checking
        );
    }

    #[test]
    fn an_offer_becomes_available_with_its_notes_and_the_check_time() {
        for manual in [true, false] {
            let next = apply(&UpdateStatus::default(), found("0.1.12"), manual);
            assert_eq!(next.phase, Phase::Available);
            assert_eq!(
                next.offer.as_ref().map(|o| o.notes.as_str()),
                Some("- Faster")
            );
            assert_eq!(next.last_checked.as_deref(), Some("2026-11-25T12:00:00Z"));
            assert_eq!(next.error, None);
        }
    }

    #[test]
    fn nothing_new_clears_the_offer() {
        let available = in_phase(Phase::Available, true);
        let manual = apply(&available, none(), true);
        assert_eq!(manual.phase, Phase::UpToDate);
        assert_eq!(manual.offer, None);
        assert_eq!(manual.last_checked.as_deref(), Some("2026-11-25T12:00:00Z"));
        let automatic = apply(&available, none(), false);
        assert_eq!(automatic.phase, Phase::Idle);
        assert_eq!(automatic.offer, None);
    }

    #[test]
    fn a_failed_check_shows_only_when_manual() {
        let checking = in_phase(Phase::Checking, false);
        let manual = apply(&checking, UpdateEvent::CheckFailed, true);
        assert_eq!(manual.phase, Phase::CheckFailed);
        assert_eq!(manual.error, Some(FailureKind::Offline));
        // The last successful time stays for "Couldn't check … last checked …".
        assert_eq!(manual.last_checked, checking.last_checked);
        let available = in_phase(Phase::Available, true);
        assert_eq!(
            apply(&available, UpdateEvent::CheckFailed, false),
            available
        );
    }

    #[test]
    fn a_check_never_replaces_a_download_or_an_install() {
        for phase in [Phase::Downloading, Phase::ReadyOnQuit, Phase::Installing] {
            let busy = in_phase(phase, true);
            assert_eq!(apply(&busy, UpdateEvent::CheckStarted, true), busy);
            assert_eq!(apply(&busy, UpdateEvent::CheckFailed, true), busy);
            for event in [found("0.1.13"), none()] {
                let next = apply(&busy, event, true);
                assert_eq!(next.phase, phase);
                assert_eq!(next.offer, busy.offer);
                assert_eq!(next.last_checked.as_deref(), Some("2026-11-25T12:00:00Z"));
            }
        }
    }

    #[test]
    fn install_now_downloads_with_progress_then_installs() {
        let available = in_phase(Phase::Available, true);
        let downloading = apply(&available, UpdateEvent::DownloadStarted, true);
        assert_eq!(downloading.phase, Phase::Downloading);
        assert_eq!(downloading.progress, None);
        let step = Progress {
            downloaded: 10,
            total: Some(100),
            percent: Some(10),
        };
        let stepped = apply(&downloading, UpdateEvent::DownloadProgress(step), true);
        assert_eq!(stepped.progress, Some(step));
        let installing = apply(&stepped, UpdateEvent::InstallStarted, true);
        assert_eq!(installing.phase, Phase::Installing);
        assert_eq!(installing.progress, None);
        assert_eq!(installing.offer, available.offer);
    }

    #[test]
    fn install_on_quit_holds_the_bytes_and_can_still_install_now() {
        let downloading = apply(
            &in_phase(Phase::Available, true),
            UpdateEvent::DownloadStarted,
            true,
        );
        let ready = apply(&downloading, UpdateEvent::Downloaded, true);
        assert_eq!(ready.phase, Phase::ReadyOnQuit);
        assert_eq!(ready.progress, None);
        assert_eq!(
            apply(&ready, UpdateEvent::InstallStarted, true).phase,
            Phase::Installing
        );
    }

    #[test]
    fn failures_keep_the_offer_for_retry() {
        let downloading = in_phase(Phase::Downloading, true);
        let failed = apply(&downloading, UpdateEvent::DownloadFailed, true);
        assert_eq!(failed.phase, Phase::InstallFailed);
        assert_eq!(failed.error, Some(FailureKind::Download));
        assert_eq!(failed.offer, downloading.offer);
        let installing = in_phase(Phase::Installing, true);
        let failed = apply(&installing, UpdateEvent::InstallFailed, true);
        assert_eq!(failed.phase, Phase::InstallFailed);
        assert_eq!(failed.error, Some(FailureKind::Verify));
        // Retry starts the download again and clears the message.
        let retry = apply(&failed, UpdateEvent::DownloadStarted, true);
        assert_eq!(retry.phase, Phase::Downloading);
        assert_eq!(retry.error, None);
    }

    #[test]
    fn a_download_needs_an_offer() {
        let idle = UpdateStatus::default();
        assert_eq!(apply(&idle, UpdateEvent::DownloadStarted, true), idle);
    }

    #[test]
    fn skip_clears_the_offer() {
        let next = apply(
            &in_phase(Phase::Available, true),
            UpdateEvent::Skipped,
            true,
        );
        assert_eq!(next.phase, Phase::Idle);
        assert_eq!(next.offer, None);
        assert!(!shows_badge(&next));
    }

    #[test]
    fn the_badge_follows_an_offer_that_was_not_skipped() {
        assert!(shows_badge(&in_phase(Phase::Available, true)));
        assert!(shows_badge(&in_phase(Phase::Downloading, true)));
        assert!(!shows_badge(&UpdateStatus::default()));
        let skipped = UpdateStatus {
            phase: Phase::Available,
            offer: Some(offer("0.1.12", true)),
            ..UpdateStatus::default()
        };
        assert!(!shows_badge(&skipped));
    }

    #[test]
    fn the_status_serialises_in_snake_case() {
        let value = serde_json::to_value(in_phase(Phase::ReadyOnQuit, false)).unwrap();
        assert_eq!(value["phase"], "ready_on_quit");
        let failed = UpdateStatus {
            error: Some(FailureKind::Verify),
            ..UpdateStatus::default()
        };
        assert_eq!(serde_json::to_value(failed).unwrap()["error"], "verify");
    }

    #[test]
    fn check_times_are_rfc3339_utc() {
        assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339_utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(rfc3339_utc(1_800_000_000), "2027-01-15T08:00:00Z");
        assert_eq!(rfc3339_utc(1_798_761_599), "2026-12-31T23:59:59Z");
    }
}
