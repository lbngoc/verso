# Changelog

All notable changes to Verso are listed here. The release pipeline publishes the section that matches
the release tag, so a tag needs its section before it is pushed.

## Unreleased

## 0.1.4 — 2026-11-20

A refreshed interface: a new logo, a settings window, and a faster way to translate and listen.

### Added

- A settings window with a grouped sidebar and cards, a shortcut recorder, and an **About** section.
- A small status tooltip and a result popup that appear next to the selection; **Speak** reads the selection
  aloud, and the quick panel can prefill from the clipboard.
- A colour choice in **Settings › Appearance**: the Verso teal or your system accent.
- An update window that shows the release notes and lets you skip or install.
- The tray icon opens the quick panel on a click; the menu (with Settings) is on a right click.
- A stepped first run that walks through the permission and the shortcuts.

### Changed

- A new Verso logo for the app and the tray.
- New installs use `Alt+Shift+Z` (Translate), `Alt+Shift+S` (Speak) and `Alt+Shift+T` (quick panel; it was
  `Alt+Shift+Cmd+T` on macOS and `Ctrl+Alt+Shift+T` on Windows). Updates never change a shortcut you saved;
  **Settings › Shortcuts** has a reset to the new defaults.
- Pressing Translate while the quick panel is open now closes the panel instead of doing nothing.
- The Homebrew cask is installed by its full name, `brew install --cask lbngoc/verso/verso`.

### Fixed

- The result popup now takes keys on macOS without taking focus away from the app you are reading.
- Closing the popup no longer crashes on macOS 26.
- On Windows the quick panel stays open when Translate finds no selection, and the selects in Settings no
  longer clip their text.
- Language and text size changes apply at once in the popup and the panel.

## 0.1.3 — 2026-11-13

The permission step also opens for upgrades from 0.1.0 and 0.1.1.

### Fixed

- Upgrading from a build that predates the version check now opens the guided Accessibility step at startup
  when the grant is missing, instead of waiting for the first failed capture.

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
