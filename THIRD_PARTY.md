# Third-Party Notices

Verso bundles or ports code from the following projects. The licenses below are the ones
declared by each upstream project at the time of porting (2026-11).

## translate-shell (Unlicense)

- Upstream: <https://github.com/soimort/translate-shell>
- Ported logic: `include/Translators/GoogleTranslate.awk` (gtx request shape, response parsing),
  `include/Translators/BingTranslator.awk` (session bootstrap, `ttranslatev3` request),
  `include/LanguageData.awk` (locale table, converted to the app's bundled language list).
- The ports are Rust re-implementations; no AWK code is executed at runtime.
- License: Unlicense (public domain). <https://unlicense.org/>

## tauri-nspanel

- Upstream: <https://github.com/ahkohd/tauri-nspanel>
- Used (when the pinned build is enabled) to convert the popup and quick windows to
  non-activating `NSPanel`s on macOS.
- License: MIT.

## tauri-plugin-liquid-glass

- Upstream: <https://github.com/cloudflyer-project/tauri-plugin-liquid-glass>
- Evaluated for the macOS 26 Liquid Glass surface. If adopted, the plugin is used as-is.
- License: MIT.

## Fluent UI System Icons

- Upstream: <https://github.com/microsoft/fluentui-system-icons>
- Windows icon set used by the Fluent stylesheet (`src/lib/icons/`).
- License: MIT.

## whatlang

- Upstream: <https://github.com/greyblake/whatlang-rs>
- Local language detection (`whatlang` crate).
- License: MIT.

## tts

- Upstream: <https://github.com/ndarilek/tts-rs>
- Text-to-speech on macOS (AVSpeechSynthesizer) and Windows (WinRT SpeechSynthesizer).
- License: MIT.

## screenTranslate

- Upstream: <https://github.com/nobiyou/screenTranslate>
- Clean-room reference only: the selection-capture (`TextGrabber`) and pasteboard snapshot
  (`PasteboardSnapshot`) behaviour was studied and re-implemented; no code is copied.

## Application dependencies

Verso links the direct dependencies below; every one is MIT or Apache-2.0, compatible with the MIT source
licence. Transitive dependencies are pinned in `Cargo.lock` and `package-lock.json`; a scan finds no
GPL/AGPL code. MPL-2.0 crates (`cssparser`, `selectors`, `dtoa-short`, `option-ext`) are used unmodified as
dependencies; the MPL covers their own files only.

| Rust crate | Licence |
|---|---|
| tauri, tauri-build, tauri-plugin-autostart, tauri-plugin-global-shortcut, tauri-plugin-log, tauri-plugin-opener, tauri-plugin-single-instance, tauri-plugin-updater | Apache-2.0 OR MIT |
| async-trait, dirs, log, parking_lot, percent-encoding, rand, reqwest, serde, serde_json, sys-locale, thiserror, url, windows | MIT OR Apache-2.0 |
| tokio, tts, whatlang, winreg | MIT |

| npm package | Licence |
|---|---|
| @tauri-apps/api, @tauri-apps/cli | Apache-2.0 OR MIT |
| @sveltejs/vite-plugin-svelte, @types/node, @eslint/js, eslint, eslint-plugin-svelte, globals, prettier, prettier-plugin-svelte, svelte, svelte-check, typescript-eslint, vite, vitest | MIT |
| typescript | Apache-2.0 |
