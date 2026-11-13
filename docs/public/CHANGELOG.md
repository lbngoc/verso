# Changelog

All notable changes to Verso are listed here. The release pipeline publishes the section that matches
the release tag, so a tag needs its section before it is pushed.

## 0.1.2 — 2026-11-13

Stable signing identity for the preview.

### Changed

- macOS builds are signed with the same code-signing identity on every release, so the Accessibility
  permission now survives updates. Upgrading from 0.1.0 or 0.1.1 asks for it once more, and Verso opens the
  guided permission step itself.
- The source is public at <https://github.com/lbngoc/verso> under the MIT licence; the release pipeline now
  runs on the public repository.

## 0.1.1 — 2026-11-11

Update-path verification release. No feature changes.

### Notes

- Confirms that 0.1.0 finds, verifies and installs a newer signed build, and that a tampered manifest is
  refused without touching the installed version.

## 0.1.0 — 2026-11-10

First public release for macOS and Windows. The builds are not code-signed yet.

### Added

- Translate the selected text with a global shortcut (default `Alt+Shift+T`), shown in a popup that can
  copy the result, speak it and expand.
- Quick input panel (default `Alt+Shift+Cmd+T` on macOS, `Ctrl+Alt+Shift+T` on Windows) for text typed
  directly into Verso.
- Google and Microsoft (Bing) engines, with automatic fallback when one fails, and an engine-order
  setting.
- Menu bar / notification-area icon with a busy state, an About surface and a two-step onboarding.
- English and Vietnamese interface, light and dark themes.
- Installers: macOS dmg for Apple Silicon and Intel, Windows x64 NSIS setup.
- Self-update: a daily check you can turn off, **Check now**, and a signed download that is verified
  before it replaces the installed version.
- **Report a problem…** from the tray or About, with the version, OS, engine order and log tail prefilled
  into the public issue form; **Copy diagnostics** for the full redacted log.

### Notes

- Builds are unsigned for now: macOS shows the first-launch warning and Windows SmartScreen asks for
  confirmation. `INSTALL.md` walks through both.
- On macOS, Verso needs the Accessibility permission before it can read the selected text.
