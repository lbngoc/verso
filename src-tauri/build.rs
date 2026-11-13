use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    // tauri-build embeds icons/icon.ico in the Windows resources but does not ask Cargo to rerun
    // when it changes, so a new app icon would otherwise keep the old one in cached builds.
    println!("cargo:rerun-if-changed=icons/icon.ico");

    // The About section shows the build date next to the version (spec 009, contracts).
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    println!("cargo:rustc-env=VERSO_BUILD_DATE={seconds}");

    tauri_build::build()
}
