fn main() {
    // tauri-build embeds icons/icon.ico in the Windows resources but does not ask Cargo to rerun
    // when it changes, so a new app icon would otherwise keep the old one in cached builds.
    println!("cargo:rerun-if-changed=icons/icon.ico");
    tauri_build::build()
}
