# Changelog

All notable changes to Verso are listed here. The release pipeline publishes the section that matches
the release tag, so a tag needs its section before it is pushed.

## Unreleased

Settings now looks like the system's own settings, and updates are shown right in Settings.

### Changed

- **Settings looks like System Settings on macOS and Settings on Windows:** the same text sizes and
  weights in every section, group headings above their boxes, each control on the right of the first line
  of its setting (also when a Vietnamese label wraps), the system's grey buttons at the right end of a row
  or at the bottom right of a box, and coloured section icons in the sidebar on macOS. Setup follows the
  same look, with its button at the bottom right.
- **Updates live in Settings › About:** a red badge on **About** says an update is waiting; About shows
  what is new with **Skip This Version**, **Install on Quit** and **Install Now**, the download progress,
  and when the last check succeeded. **Update available** in the tray opens it. The separate update window
  is gone.

### Fixed

- **The panel and popup stay readable over bright windows on macOS 26.** Liquid Glass could turn light
  behind them while their text stayed light in dark mode; they now keep a surface in the theme's colour,
  with the glass showing softly through.

## 0.1.10 — 2026-11-25

Verso's windows now fit on every display of a Mac with a Retina screen and an external monitor, the
panel is easier to type in, and updates are checked more often.

### Fixed

- **Windows fit on every display.** With a Retina display and an external monitor, the panel, popup,
  Settings and update window could open half their size or with their content cut off after being shown
  on the other display. They now keep their size and show all their content on either display.
- **The panel opens under the icon you click**, on the menu bar of the display you clicked, right below it
  (it could open in the wrong place, or too low on an external display above the built-in one).
- **The update window shows its buttons** again; its bottom row was cut off.
- **Input methods:** panel keys (Enter, Tab, Esc…) no longer act in the middle of a composition, and Verso
  becomes the active app while you type in the panel. With VietTelex, set Verso to **Gạch chân** if
  accents get lost (see Troubleshooting).

### Changed

- **The panel's original text and translation look alike:** same box, same system text size (13 px on
  macOS, 14 px on Windows), scaled together by the text size buttons.
- **A status bar at the bottom of the panel and popup:** **Copy** and **Speak** with their keys on the left;
  on the right, what applies now — in the panel the automatic-translation mode (click to switch), in the
  popup the Esc key. Engines are shown by name (Google, Microsoft), and Verso no longer reports that one
  service answered after another failed.
- **Translate as you type** in **Settings › Popup & panel**: Fast, **Slow** (the default: fewer requests,
  so the free services make you wait less often) or Off (press Enter). Enter always translates at once.
- **Update checks** run about every 4 hours (they were daily and could slip while the Mac slept); a failed
  check is tried again after 30 minutes.

## 0.1.9 — 2026-11-25

Screen translation now shows its result in the translation panel, where you can correct the text, and
the panel, Settings and popup are tidied up.

### Changed

- **Screen translation opens the translation panel.** After you drag around some text, the panel opens next
  to the area with the text Verso read, already being translated. Fix a misread word and the translation
  follows; **Copy** and **Speak** work as usual, and **Esc** returns you to the app you were in. Closing
  the panel while the text is still being read cancels the translation: nothing is sent.
- **The panel opens where you would look for it:** under the tray icon when you click it, upper-centre of
  the screen for `⌥⇧T`, and beside the selected area for a screen result.
- **Screen translation options moved into the existing sections:** the switch and recognition language are
  in **General**, the shortcut is in **Shortcuts**, and the Screen Recording grant stays in **Permissions**.
- The Accessibility and Screen Recording rows in **Permissions** line up with every other setting.

### Added

- A **Copy** button on the original text in the panel.

### Fixed

- The selection popup no longer flashes the previous translation when it opens, and no longer jumps
  between "Translating…" and the result.
- On Windows, screen translation found no text when you selected a single line closely (the picture was
  too thin for the system's recogniser). It now reads thin selections as well as larger ones.

## 0.1.8 — 2026-11-23

Verso can now translate text on the screen that cannot be selected, and the update window and Permissions
list are fixed up.

### Added

- **Screen translation** (optional, off by default): press `⌥⇧X` / `Alt+Shift+X`, drag around some text, and
  the translation appears next to it, with its engine and a Copy and Speak button. The picture is read in
  memory by the system's recogniser (Vision on macOS 14+, Windows OCR on Windows) and is never saved or sent
  anywhere. Turn it on, change its shortcut or choose the recognition language in **Settings › Screen
  translation**; the tray menu gains **Translate Screen Region** while it is on. On macOS it needs the
  **Screen Recording** permission, which Verso guides you through (macOS may ask to quit and reopen Verso).
- **Settings › Permissions** now lists each permission as its own row that switches to "granted" by itself.

### Fixed

- The update progress bar stayed near 0 % while a download ran. It now follows the real download (and shows
  a moving bar when the size is unknown), reaches 100 % before "installing", and the update window is as tall
  as its content instead of leaving an empty area.
- The tray icon is larger and easier to see in the menu bar and taskbar.

## 0.1.7 — 2026-11-22

When an update leaves macOS no longer trusting Verso, setup says so and fixes it with one click; Settings
looks the same however it is opened, and several Windows focus problems are gone.

### Changed

- When an update leaves macOS no longer trusting Verso for Accessibility, the setup window now says the
  update is the reason and offers **Reset and grant again**: it clears Verso's old entry and asks again,
  so you only choose **Open System Settings** in the dialog and switch Verso on. The window notices the
  permission within a second, without a restart.

### Fixed

- Settings opened with `⌘,` from the popup or the quick panel (or from an engine error's **Change engine
  order** button) showed a transparent sidebar on macOS; it now looks the same however it is opened.
- With **Settings › Appearance › Theme** set to Light or Dark against the opposite system setting, the
  Settings sidebar and the quick panel kept the system's shade behind Verso's text on macOS; they now
  follow Verso's choice, from the first frame and when the theme is changed.
- On macOS 26 the popup's glass could take the shade of the window behind it and leave the translation
  hard to read; it now keeps a light or dark tint that matches the theme.
- On Windows, `Ctrl+,` from the popup could open Settings behind the app you were reading; the popup now
  stays until Settings is on screen and Settings keeps the focus.
- On Windows, clicking the tray icon again after closing the quick panel could show nothing: the panel
  opened and closed itself as its translation arrived. It now stays open.
- The install and troubleshooting guides no longer describe the macOS build as ad-hoc signed.

## 0.1.6 — 2026-11-21

Verso now behaves like a menu-bar app, uses less memory when idle, and checks for updates inside
**Settings › About**.

### Changed

- Verso lives in the menu bar (the notification area on Windows). It shows a Dock icon and a Cmd+Tab
  entry (a taskbar button and an Alt+Tab entry on Windows) only while setup, Settings or the update window
  is open.
- Only one main window at a time: while setup is open, Settings, the update window, the popup and the
  quick panel wait until it is finished. The update window can still open next to Settings.
- **Check for Updates…** checks right in **Settings › About**: it says when you already have the newest
  version or when the update service cannot be reached, and opens the update window only when there is a
  new version to install. The check gives up after 15 seconds instead of waiting much longer offline.
- The quick panel is created the first time you open it, which saves memory while Verso is idle; that
  first opening can take a moment longer.
- With the Vietnamese interface, language names, the tray menu, the tray tooltip and window titles are
  in Vietnamese too.
- The links in **Settings › About** open the public `verso-translate` repository; **Terms** opens the
  terms of use and a **Privacy** link was added.
- The key list in **Settings › Popup & panel** includes `⌘,` / `Ctrl+,` (Settings).
- The shortcut recorder names only the modifier keys of your system.
- The tray mark is taller with a bolder V, so it is easier to read in the menu bar and the notification
  area.

### Fixed

- The warning and success colours are slightly darker in the light theme, so their text is easier to read.
- `Ctrl+,` in the result popup on Windows opens Settings in front of the app you were using, not behind it.
- Release notes in the update window keep a long bullet together instead of splitting it.

## 0.1.5 — 2026-11-20

Polish for first-run setup, Settings and the quick panel, after trying 0.1.4 on a Mac.

### Added

- A theme choice (System, Light, Dark) on the first step of setup; it applies at once.
- Key hints in the quick panel footer: `↵` Translate, `Esc` Back or Close, `⇥` Swap and `⌘,` Settings. The
  keys that work in the current focus are highlighted. The result popup shows `Esc` Close.
- `Cmd+,` (`Ctrl+,` on Windows) opens Settings from the popup and the quick panel.

### Changed

- A missing Accessibility permission is shown as an amber warning with an icon, in setup and in
  **Settings › Permissions**; a granted one as a green badge.
- First-run setup fits its window without a scrollbar, and its main buttons have an arrow that slides on hover.
- The quick panel has more even spacing between its parts.
- The empty **Modules** section is gone from Settings.

### Fixed

- With the Settings sidebar collapsed on macOS, the toggle no longer sits under the window buttons.

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
