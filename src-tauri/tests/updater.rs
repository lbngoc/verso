//! Self-update policy tests (007 T044/T066, contracts/update-channel.md): no network, no server.

use std::time::Duration;

use verso_lib::updater::{is_newer, is_prerelease, should_check, should_offer, wait_before_check};

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
fn schedule_is_thirty_seconds_then_every_24_hours() {
    assert_eq!(wait_before_check(0), Duration::from_secs(30));
    for attempt in 1..5 {
        assert_eq!(
            wait_before_check(attempt),
            Duration::from_secs(24 * 60 * 60)
        );
    }
}
