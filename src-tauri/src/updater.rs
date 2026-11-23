//! Self-update (FR-015/016/017, contracts/update-channel.md).
//!
//! The policy lives here as pure functions so it can be tested without a server: when a check is
//! allowed (never before onboarding ends, never when the user turned it off), when a release is an
//! offer (newer than the running version, never a pre-release), and the 30 s / 24 h schedule.
//!
//! The network side is `tauri-plugin-updater`; the private key never reaches the app, and the
//! signature is verified against the built-in public key before anything is installed.

use std::sync::OnceLock;
use std::time::Duration;

use parking_lot::Mutex;
use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::Update;

use crate::commands::AppState;

/// First check 30 s after start, then once a day (contracts/update-channel.md).
pub const FIRST_CHECK_DELAY: Duration = Duration::from_secs(30);
pub const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

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

static PENDING: OnceLock<Mutex<Option<Update>>> = OnceLock::new();
static PENDING_BYTES: OnceLock<Mutex<Option<Vec<u8>>>> = OnceLock::new();

fn pending() -> &'static Mutex<Option<Update>> {
    PENDING.get_or_init(|| Mutex::new(None))
}

fn pending_bytes() -> &'static Mutex<Option<Vec<u8>>> {
    PENDING_BYTES.get_or_init(|| Mutex::new(None))
}

/// How long to wait before check number `attempt` (0 = the first one).
pub fn wait_before_check(attempt: u32) -> Duration {
    if attempt == 0 {
        FIRST_CHECK_DELAY
    } else {
        CHECK_INTERVAL
    }
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

async fn check(app: &AppHandle, manual: bool) -> Result<Option<UpdateInfo>, String> {
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
    *pending().lock() = Some(update);
    log::info!("update available: {}", info.version);
    Ok(Some(info))
}

fn announce(app: &AppHandle, info: &UpdateInfo) {
    let _ = app.emit("update-available", info);
    crate::tray::set_update_available(app, Some(&info.version));
}

/// Starts the automatic schedule. Nothing is requested before the onboarding choice is saved.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut attempt = 0u32;
        loop {
            // Until onboarding ends, retry often: the first check should follow the user's choice by
            // about 30 s, not wait for a start that already happened. Afterwards it is 30 s then daily.
            let onboarding = app
                .state::<AppState>()
                .settings
                .get()
                .first_run_engine_choice_at
                .is_some();
            let wait = if onboarding {
                wait_before_check(attempt)
            } else {
                FIRST_CHECK_DELAY
            };
            tokio::time::sleep(wait).await;

            if !auto_check_enabled(&app) {
                if onboarding {
                    attempt = attempt.saturating_add(1);
                }
                continue;
            }
            attempt = attempt.saturating_add(1);

            match check(&app, false).await {
                Ok(Some(info)) => announce(&app, &info),
                Ok(None) => log::debug!("update check: up to date"),
                Err(error) => log::warn!("update check failed: {error}"),
            }
        }
    });
}

/// "Check now": always requests, whatever the preference says, and always reports the result.
#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let info = check(&app, true).await?;
    if let Some(info) = &info {
        announce(&app, info);
    } else {
        crate::tray::set_update_available(&app, None);
    }
    Ok(info)
}

/// What the update window shows while a download runs (data-model `UpdateProgress`).
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
    let emitter = app.clone();
    let mut tracker = ProgressTracker::default();
    move |chunk_len, total| {
        if let Some(progress) = tracker.on_chunk(chunk_len, total) {
            let _ = emitter.emit("update-progress", progress);
        }
    }
}

/// Downloads, verifies and applies the offered release, then relaunches.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let update = pending()
        .lock()
        .clone()
        .ok_or_else(|| "no update is available".to_string())?;
    // "Install on Quit" may already hold verified bytes; reuse them instead of downloading twice.
    let held = pending_bytes().lock().take();
    let result = match held {
        Some(bytes) => update.install(bytes),
        None => update
            .download_and_install(progress_reporter(&app), || {})
            .await
            .map(|_| ()),
    };
    result.map_err(|error| {
        // The UI shows "update could not be verified"; the detail belongs in the log.
        log::warn!("update could not be installed: {error}");
        error.to_string()
    })?;

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
    let bytes = update
        .download(progress_reporter(&app), || {})
        .await
        .map_err(|error| {
            log::warn!("update could not be downloaded: {error}");
            error.to_string()
        })?;
    *pending_bytes().lock() = Some(bytes);
    log::info!("update verified; it will be installed when Verso quits");
    Ok(())
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

/// "Skip This Version": remember it, forget the offer and clear the tray item.
#[tauri::command]
pub fn skip_update_version(app: AppHandle, version: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let profile = state
        .settings
        .update(&json!({ "skipped_update_version": version }))
        .map_err(|error| error.to_string())?;
    *pending().lock() = None;
    *pending_bytes().lock() = None;
    crate::tray::set_update_available(&app, None);
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

    #[test]
    fn schedule_is_thirty_seconds_then_daily() {
        assert_eq!(wait_before_check(0), Duration::from_secs(30));
        assert_eq!(wait_before_check(1), Duration::from_secs(86_400));
        assert_eq!(wait_before_check(7), Duration::from_secs(86_400));
    }
}
