//! Self-update policy tests (007 T044/T066, contracts/update-channel.md): no network, no server.

use std::time::Duration;

use verso_lib::updater::{
    is_newer, is_prerelease, should_check, should_offer, CheckSchedule, Progress, ProgressTracker,
};

#[test]
fn newer_only_version_rule() {
    assert!(is_newer("0.1.0", "0.1.1"));
    assert!(is_newer("0.1.9", "0.2.0"));
    assert!(is_newer("0.9.9", "1.0.0"));
    assert!(!is_newer("0.1.0", "0.1.0"));
    assert!(!is_newer("0.2.0", "0.1.9"));
    assert!(!is_newer("0.1.0", "0.1"));
    assert!(!is_newer("", "0.1.1"));
}

#[test]
fn pre_release_is_ignored() {
    assert!(!should_offer("0.1.0", "0.1.1-rc.1"));
    assert!(!should_offer("0.1.0-rc.1", "0.1.0-rc.2"));
    assert!(is_prerelease("0.1.1-beta"));
    assert!(should_offer("0.1.0", "0.1.1"));
}

#[test]
fn check_updates_false_makes_no_request() {
    // The scheduler consults this before touching the network.
    assert!(!should_check(true, false));
    assert!(!should_check(false, true));
    assert!(should_check(true, true));
}

#[test]
fn no_check_before_onboarding_finishes() {
    // Onboarding ends when the engine choice is saved; until then `check_updates` is not honoured.
    assert!(!should_check(false, true));
}

#[test]
fn the_schedule_checks_every_four_hours_of_wall_clock_time() {
    let start = std::time::SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
    let mut schedule = CheckSchedule::new();
    assert!(schedule.due(start));
    schedule.succeeded(start, 0);
    assert!(!schedule.due(start + Duration::from_secs(3 * 3600)));
    assert!(schedule.due(start + Duration::from_secs(4 * 3600)));
}

// ---- spec 004 (T013): the progress bar ----

#[test]
fn chunk_sizes_add_up_to_a_running_percent() {
    // The 0.1.7 bug: each chunk's length was read as the total, so the bar stayed near 0 %.
    let mut tracker = ProgressTracker::default();
    let percents: Vec<u32> = [10usize, 20, 70]
        .into_iter()
        .filter_map(|chunk| tracker.on_chunk(chunk, Some(100)))
        .map(|progress| progress.percent.unwrap())
        .collect();
    assert_eq!(percents, vec![10, 30, 100]);
}

#[test]
fn percent_never_goes_back_and_is_capped_at_100() {
    let mut tracker = ProgressTracker::default();
    let mut last = 0;
    for chunk in [30usize, 30, 30, 30, 30] {
        if let Some(progress) = tracker.on_chunk(chunk, Some(100)) {
            let percent = progress.percent.unwrap();
            assert!(percent > last && percent <= 100);
            last = percent;
        }
    }
    assert_eq!(last, 100);
}

#[test]
fn at_most_one_report_per_percent_step() {
    let mut tracker = ProgressTracker::default();
    let reports = (0..1000)
        .filter_map(|_| tracker.on_chunk(1, Some(1000)))
        .count();
    assert_eq!(reports, 101, "0 % to 100 %, each step once");
    // A chunk that does not move the percent reports nothing.
    let mut tracker = ProgressTracker::default();
    assert!(tracker.on_chunk(50, Some(10_000)).is_some());
    assert!(tracker.on_chunk(40, Some(10_000)).is_none());
}

#[test]
fn an_unknown_length_reports_without_a_percent_every_256_kib() {
    for total in [None, Some(0)] {
        let mut tracker = ProgressTracker::default();
        let reports: Vec<Progress> = (0..10)
            .filter_map(|_| tracker.on_chunk(100 * 1024, total))
            .collect();
        // 1,000 KiB in 100 KiB chunks: first chunk, then every 256 KiB or more.
        assert!(
            reports.len() >= 3 && reports.len() <= 4,
            "{}",
            reports.len()
        );
        assert!(reports
            .iter()
            .all(|r| r.total.is_none() && r.percent.is_none()));
        assert_eq!(reports[0].downloaded, 100 * 1024);
    }
}
