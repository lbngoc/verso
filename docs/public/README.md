# Verso Translate

**Tiếng Việt:** [README_vi.md](README_vi.md)

> **Preview** — `0.1.0` is the first public preview for macOS and Windows. It updates itself; Linux, signed
> builds and winget arrive in ordinary updates afterwards. Homebrew is available now.

Verso is a small translator that lives in the menu bar (macOS) or the notification area (Windows). Select
text in any app and press `Alt+Shift+T`: Verso translates it and shows the result in a popup that can copy
or speak it. Text you type yourself goes through the quick panel.

- **Engines**: Google Translate and Microsoft Translator (Bing), no account or API key needed, with
  automatic fallback when one fails.
- **Automatic source language**, dozens of target languages, English or Vietnamese interface.
- **Light and dark themes**; the tray mark follows the system.
- **Updates itself** once you agree to the daily check, with a signed, verified download.
- **Report a problem** from the tray or Settings, with the log tail prefilled into the issue form.
- **No telemetry** and no account. See [PRIVACY.md](PRIVACY.md).

## Install

### Homebrew (macOS)

```sh
brew tap lbngoc/verso
brew install --cask verso
```

The cask clears the download quarantine flag after installing, so the preview opens without the Gatekeeper
prompt. `brew` does not need to upgrade it (the cask is marked `auto_updates`): Verso updates itself.

### Direct download

The latest installers are on this repository's [Releases](../../releases/latest) page:

| System | File |
|---|---|
| macOS 13 or later, Apple Silicon | `Verso_<version>_aarch64.dmg` |
| macOS 13 or later, Intel | `Verso_<version>_x64.dmg` |
| Windows 10 or 11, 64-bit | `Verso_<version>_x64-setup.exe` |

Every release also publishes `SHA256SUMS.txt` with the SHA-256 of each file.

**First launch on macOS.** The preview is ad-hoc signed but not notarized, so macOS blocks it once with
"Apple could not verify 'Verso' is free of malware". Dismiss it with **Done**, then open **System Settings ›
Privacy & Security** and scroll to **Security**: click **Open Anyway** next to Verso and enter your login
password. The button is offered for about an hour after the blocked attempt. On macOS 13 and 14 you can also
right-click `Verso` in `Applications` and choose **Open**. If the button does not appear, the Terminal
fallback and the checksum commands are in [INSTALL.md](INSTALL.md).

**Windows.** SmartScreen shows "Windows protected your PC" once: choose **More info**, then **Run anyway**.
Winget is submitted after the preview; until then take the x64 setup and let the app update itself.

Whichever channel installed Verso, it keeps itself up to date.

## Documentation

- [INSTALL.md](INSTALL.md) — install on macOS and Windows, checksums, permissions, updates
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) — blocked engines, shortcuts, logs, updates, reporting
- [PRIVACY.md](PRIVACY.md) — which engine receives your text
- [TERMS.md](TERMS.md) — terms for the binaries
- [CHANGELOG.md](CHANGELOG.md) — what each release contains
- [THIRD_PARTY.md](THIRD_PARTY.md) — third-party notices
- [SECURITY.md](SECURITY.md) — reporting a problem privately

## Screenshots

Screenshots of the popup, the quick panel and the Settings window will be added after the first public
release.

## Project status

The source is open at [github.com/lbngoc/verso](https://github.com/lbngoc/verso) under the MIT licence;
issues and pull requests are welcome there. This repository holds the published documents and the release
downloads. The terms that apply to the binaries are in [TERMS.md](TERMS.md).
