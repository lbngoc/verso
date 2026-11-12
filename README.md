# Verso

Verso is a small menu-bar (macOS) / notification-area (Windows) translator. Select text in any app and
press `Alt+Shift+T`: Verso translates it with Google or Microsoft (Bing) and shows the result in a popup
that can copy or speak it. Text you type yourself goes through the quick panel.

- Google Translate and Microsoft Translator (Bing), no account or API key, with automatic fallback.
- Automatic source language, dozens of target languages, English or Vietnamese interface.
- Tray icon, global shortcuts, light and dark themes, launch at login.
- Self-update with a signed, verified download and an explicit consent choice.
- **Report a problem** with the log tail prefilled into the public issue form.
- No telemetry and no account.

## Install

Users install from the [releases](https://github.com/lbngoc/verso-translate/releases/latest), or on macOS
with Homebrew:

```sh
brew tap lbngoc/verso
brew install --cask verso
```

The guides (install, troubleshooting, privacy, terms) live in
[lbngoc/verso-translate](https://github.com/lbngoc/verso-translate).

## Build from source

Requirements: Node 20+, Rust 1.80+ with `rustfmt` and `clippy`, and the platform toolchain (Xcode command
line tools on macOS, Visual Studio Build Tools with the C++ workload on Windows). Windows also needs
WebView2, which ships with Windows 10/11.

```sh
npm ci
npm run tauri dev
```

Checks and tests:

```sh
npm run check
npm run lint
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Installers: `npm run tauri build` (macOS dmg/app) or `npm run build:windows` (cross-build from macOS).

## Layout

| Path | What |
|---|---|
| `src-tauri/src/core/` | engine-agnostic core: pipeline, registry, settings, capture, languages |
| `src-tauri/src/engines/` | Google and Bing adapters |
| `src-tauri/src/platform/` | macOS and Windows native services |
| `src-tauri/src/updater.rs`, `report.rs` | self-update and the prefilled issue report |
| `src/` | Svelte UI: popup, quick panel, settings, onboarding |
| `docs/public/` | user-facing documents published to the release repository |
| `scripts/` | release and verification scripts |
| `.github/workflows/` | CI and the release pipeline |

## Releasing

A version tag (`vX.Y.Z`) runs `.github/workflows/release.yml`: the version guard, the checks, the macOS
(arm64 + x64) and Windows builds, then the publication to the public documents repository with checksums,
release notes and `latest.json`. The updater key and the code-signing certificate live only in the
repository secrets. See [CONTRIBUTING.md](CONTRIBUTING.md) for the checks a change must pass.

## License

MIT — see [LICENSE](LICENSE). The compiled binaries are covered by the terms in
[docs/public/TERMS.md](docs/public/TERMS.md). Third-party notices are in [THIRD_PARTY.md](THIRD_PARTY.md).

## Security

Report vulnerabilities privately; see [SECURITY.md](SECURITY.md).
