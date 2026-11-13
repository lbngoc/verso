# Troubleshooting Verso

Written for **Verso 0.1.3** (macOS 13 or later; Windows 10 or 11, 64-bit).

## The translation service is busy or refuses requests

Verso uses the public Google Translate and Microsoft Translator endpoints, which have no account and no
API key. Under heavy use they can rate-limit or block a network:

- **"The translation service is busy. Try again shortly or change the engine order."** — a temporary rate
  limit (HTTP 429). Wait a minute, or put the other engine first.
- **"The translation service is refusing requests from your network for now. Use another engine or try
  again later."** — the service blocked the network. The other engine usually still works; a VPN or another
  network can help.

Change the order in **Settings › Engines** (tray menu › Settings…): move the working service to the top, or
turn one off. With both on, Verso falls back automatically and tells you which service answered.

## macOS says it could not verify Verso

The builds are ad-hoc signed but not notarized, so Gatekeeper blocks the first launch with "Apple could
not verify 'Verso' is free of malware". Dismiss it with **Done**, then open **System Settings › Privacy &
Security** and scroll to **Security**: a line about Verso has an **Open Anyway** button. Click it, enter
your login password, and open Verso again. The button appears for about an hour after the blocked attempt;
if it is not there, double-click Verso once more and look again immediately.

If the button does not appear at all, run this in Terminal and open Verso again:

```sh
xattr -dr com.apple.quarantine /Applications/Verso.app
```

That removes the download quarantine flag; it does not change the app. (macOS 13 and 14 also accept
right-clicking Verso in Applications and choosing **Open**.)

## The shortcut does nothing

- Make sure Verso is running: the two-arrow mark is in the menu bar or the notification area.
- The combination may belong to another app. Change it in **Settings › Shortcuts**; a conflict is reported
  when you save.
- On macOS, translating the selected text needs the **Accessibility** permission: **System Settings ›
  Privacy & Security › Accessibility** must list Verso and have it switched on. After replacing
  `Verso.app` with a new build, macOS may keep the old entry switched on while the new build is not
  trusted: remove the entry with minus, add the new app with plus, and switch it on.
- The quick panel (`Alt+Shift+Cmd+T` on macOS, `Ctrl+Alt+Shift+T` on Windows) works without Accessibility;
  use it to check the engines while the selection path is being fixed.

## Nothing is captured, or the popup says no text was selected

- Some apps (password fields, secure input) never expose their selection. Use the quick panel there.
- Select the text first, then press the shortcut; Verso reads the current selection at that moment.
- If macOS shows the Accessibility entry switched on but Verso still reports it as missing, remove and
  re-add Verso as described above.

## Where the logs are

- macOS: `~/Library/Logs/app.versotranslate/verso.log`
- Windows: `%LOCALAPPDATA%\app.versotranslate\logs\verso.log`

Settings live in `~/Library/Application Support/app.versotranslate/settings.json` (macOS) and
`%APPDATA%\app.versotranslate\settings.json` (Windows). At the default level the logs hold engine ids,
counts and error kinds, not the translated text.

## Reporting a problem

The fastest route is inside the app: **Report a problem…** in the tray menu, or **Settings › About ›
Report a problem…**. The browser opens the public issue form with the version, the operating system, the
engine order and the last lines of the log already filled in; review it, add what you did and what you
expected, and submit. **Copy diagnostics** copies a fuller text to the clipboard if the log tail was cut.

If the form does not open, open an issue on this repository with:

- the Verso version (tray menu › About Verso, or **Settings › About**) and the OS version;
- what you did, what you expected and what happened;
- the last lines of the log around the failure (remove anything personal first).

Security problems go through [SECURITY.md](SECURITY.md), not a public issue.

## Updates

- **No update is offered.** The automatic check runs about 30 seconds after setup finishes and then once a
  day; press **Check now** in **Settings › About** to ask immediately. The check is off if you turned off
  **Check for updates automatically** during setup.
- **The update could not be verified.** The download did not match the signature that ships in the app,
  so nothing was replaced. Try again; if it keeps happening, report it rather than downloading an
  installer from anywhere else.
- **The app did not come back after installing.** On macOS, open Verso again from `Applications`. On
  Windows, the installer closes the app while it replaces it and starts the new version itself.
- **Accessibility stopped working after an update (macOS).** Builds are ad-hoc signed, so macOS may tie
  the grant to the previous build: remove Verso in **System Settings › Privacy & Security ›
  Accessibility** and add the new `Verso.app` again, as in [INSTALL.md](INSTALL.md).
