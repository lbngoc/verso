# Installing Verso

Written for **Verso 0.1.8** (macOS 13 or later; Windows 10 or 11, 64-bit). The macOS build is signed with
Verso's own (self-signed) certificate but not notarized, and the Windows build is unsigned, so both systems ask for one confirmation on the first
launch; the steps below cover it.

Take the installer from the [latest release](../../releases/latest) page. The file names carry the version:

| System | File |
|---|---|
| macOS, Apple Silicon (M1 and later) | `Verso_0.1.5_aarch64.dmg` |
| macOS, Intel | `Verso_0.1.5_x64.dmg` |
| Windows, 64-bit | `Verso_0.1.5_x64-setup.exe` |

## Homebrew (macOS)

```sh
brew tap lbngoc/verso
brew install --cask lbngoc/verso/verso
```

The cask removes the download quarantine flag after installing, so the first-launch warning below does not
appear. Homebrew does not upgrade the app (the cask is marked `auto_updates`): Verso updates itself.

## Verify the download (recommended)

Each release lists the SHA-256 of every file in `SHA256SUMS.txt`.

macOS:

```sh
shasum -a 256 Verso_0.1.5_aarch64.dmg
```

Windows (PowerShell):

```powershell
Get-FileHash .\Verso_0.1.5_x64-setup.exe -Algorithm SHA256
```

The value must match the line for that file in `SHA256SUMS.txt`. If it does not, download the file again
from the release page.

## macOS

If you installed with Homebrew, skip steps 1–2: start Verso from `Applications`; the cask already cleared the
quarantine flag, and Accessibility is asked for as in step 3.

1. Open the `.dmg` and drag **Verso** into `Applications`.
2. The first launch is blocked by Gatekeeper because the build is not notarized; the alert reads
   "Apple could not verify 'Verso' is free of malware". Dismiss it with **Done** (not **Move to Trash**),
   then open **System Settings › Privacy & Security** and scroll to **Security**: a line about Verso has
   an **Open Anyway** button. Click it, enter your login password, then open Verso again. The button is
   offered for about an hour after the blocked attempt, so do this right away. On macOS 13 and 14 you can
   also right-click (Control-click) `Verso` in `Applications` and choose **Open**, then **Open** again.
3. Verso asks for the **Accessibility** permission the first time it needs to read a selection. Grant it in
   **System Settings › Privacy & Security › Accessibility** (switch Verso on; the first time, confirm with
   your password). Without it, translating a selection cannot read the text; the quick panel still works.
   If Verso is listed and switched on but still asks after an update, choose **Reset and grant again** in
   its setup window and switch Verso on in the list that opens (see
   [TROUBLESHOOTING.md](TROUBLESHOOTING.md)).
4. Start Verso. The mark (two arrows) appears in the menu bar. Select some text in another app and press
   `Alt+Shift+Z`; the result appears next to the selection. `Alt+Shift+T` opens the quick panel.
   Verso runs from the menu bar: it has no Dock icon while you are not using one of its windows (setup,
   Settings or the update window).

Verso starts with the system at login only if **Launch at login** is enabled in Settings.

## Windows

1. Run `Verso_0.1.5_x64-setup.exe`.
2. Windows SmartScreen shows "Windows protected your PC" because the build is unsigned. Choose **More
   info**, then **Run anyway**. The installer needs no administrator rights and installs for the current
   user.
3. Start **Verso** from the Start menu. The mark (two arrows) appears in the notification area; if it is
   hidden, open the overflow arrow next to the clock.
4. Select some text in another app and press `Alt+Shift+Z`; the result appears next to the selection.
   `Alt+Shift+T` opens the quick panel.

To remove Verso, use **Settings › Apps › Installed apps › Verso › Uninstall** (or `uninstall.exe` in
`%LOCALAPPDATA%\Verso`).

## First use

The first launch shows a two-step introduction: choose which services may receive your text (Google and
Microsoft are both on by default; at least one must stay on), then grant the Accessibility permission on
macOS (a step indicator shows 1 Services · 2 Permission). The second step offers **Check for updates automatically**, pre-checked, with a line about the
request; leave it on or turn it off. Everything can be changed later in **Settings** from the tray menu.

Shortcuts can be changed in **Settings › Shortcuts**: click a field and type the combination (macOS shows
`⌥⇧Z`), ↺ restores the default (the new defaults: Translate `⌥⇧Z`, Speak `⌥⇧S`, panel `⌥⇧T`; saved keys are never changed by an update) and ⌫ clears it. The three actions are Translate selection, Speak selection
and Quick panel. On macOS a global shortcut needs no permission beyond Accessibility; if another app owns
the combination, Verso says so and asks for another. The same card has an off-by-default option to paste
the clipboard into the panel when nothing is selected.

## Screen translation (optional)

**Settings › Screen translation** turns on a fourth action, `⌥⇧X` (`Alt+Shift+X`): the screen dims, you drag
around some text, and the translation appears next to it. It is off until you switch it on.

- **macOS 14 or later.** Verso needs the **Screen Recording** permission (**System Settings › Privacy &
  Security › Screen & System Audio Recording**). Verso opens the right pane; after you switch it on, macOS
  may ask Verso to quit and reopen before the permission works (**Quit & Reopen**, or **Restart Verso** in
  Verso's own window). macOS 13 shows the option as unavailable.
- **Windows** needs no permission, but a language with text recognition must be installed in **Settings ›
  Time & language › Language & region** (English is usually there). Without one the option says so.
- The picture is read in memory by the system's own recogniser and dropped at once: it is never saved, never
  sent anywhere; only the recognised text goes to your chosen translation service.

## Updates

Verso updates itself. When the daily check finds a newer stable release, the tray shows **Update
available**; **Check for Updates…** in **Settings › About** opens the update dialog, which renders the
release notes and offers **Install and Relaunch**, **Install on Quit** and **Skip This Version**. An
install downloads the update, verifies its signature with the key built into the app and replaces the
installed version; a download that does not verify is discarded and the current version keeps working.
Pre-release builds are never offered, a skipped version stays hidden until a newer one appears, and the
automatic check can be switched off in the dialog or in **Settings › About**.

## Where your data lives

- macOS settings: `~/Library/Application Support/app.versotranslate/settings.json`; logs:
  `~/Library/Logs/app.versotranslate/verso.log`.
- Windows settings: `%APPDATA%\app.versotranslate\settings.json`; logs:
  `%LOCALAPPDATA%\app.versotranslate\logs\verso.log`.

Nothing is stored off the device. See [PRIVACY.md](PRIVACY.md).
