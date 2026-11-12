# Privacy

Verso has no account, no analytics and no telemetry. This page states exactly what leaves your device.

## What is sent, and to whom

When you translate, Verso sends **the text you asked it to translate** — the current selection, or what you
typed in the quick panel — to the translation service that answers:

| Engine | Service | Endpoint |
|---|---|---|
| Google | Google Translate (public web endpoint, no API key) | `translate.googleapis.com` |
| Microsoft | Microsoft Translator through Bing (public web endpoint) | `www.bing.com` |

Nothing else is sent: no device or user identifier, no clipboard history, no settings, no usage data. With
both engines enabled, Verso tries the first in your engine order and sends the text to the second only when
the first fails; the popup says when a fallback answered.

The services handle the text under their own policies:

- Google: <https://policies.google.com/privacy>
- Microsoft: <https://privacy.microsoft.com/privacystatement>

If you turn an engine off in **Settings › Engines**, Verso never contacts it.

## The clipboard

Verso reads the selection directly through the system accessibility interfaces whenever it can. When an app
does not expose its selection that way, Verso may copy it with a simulated copy, read the clipboard and put
the previous clipboard contents back. If some rich formats cannot be restored, the popup says so. Verso does
not keep a clipboard history and does not send the clipboard anywhere else.

**Copy** in the popup writes the translation to the clipboard; that is the only other clipboard use.

## What stays on your device

- Settings: `~/Library/Application Support/app.versotranslate/settings.json` (macOS) or
  `%APPDATA%\app.versotranslate\settings.json` (Windows).
- Logs: `~/Library/Logs/app.versotranslate/verso.log` (macOS) or
  `%LOCALAPPDATA%\app.versotranslate\logs\verso.log` (Windows). At the default level the logs record engine
  ids, counts, timings and error kinds. Translated text and request URLs are never written at that level;
  the debug level can include redacted request URLs, never the text itself.

Verso stores no history of translations. Uninstalling removes the application; the settings and log files
above can be deleted by hand.

## Telemetry setting

No version of Verso sends telemetry. Older settings files may contain a `telemetry_opt_in` flag; the app
ignores it and writes it back as `false`.

## Update checks

The first launch offers **Check for updates automatically**, pre-checked, with an explanation of the
request. Leave it on and Verso asks `github.com` for one file, `latest.json`, about 30 seconds after setup
finishes and then once a day; turn it off (in the introduction or later in **Settings › About**) and Verso
makes no update request at all. The request is an ordinary HTTPS download of a public file: it carries no
text, no selection, no clipboard content, no settings and no identifier. GitHub sees what it sees for any
public download (an IP address and a user agent), under its own policy:
<https://docs.github.com/site-policy/privacy-policies/github-general-privacy-statement>.

**Check now** in Settings asks even when the automatic checks are off, because you asked for it.

An offered update is downloaded only when you press **Install**. The download is verified against the
public key built into the app, and only a matching, newer, signed release is applied; a failed check
leaves the installed version untouched.

## Reporting a problem

**Report a problem…** (tray menu, or Settings › About) opens the public issue form in your browser with the
version, the operating system, the engine order and the last lines of the log already filled in. The app
does not send anything itself: you see the form, can edit or clear every field, and choose whether to
submit it on GitHub.

The log text is shortened so the address stays under 6,000 characters, and your home directory is
replaced with `~` before it is placed in the address. Translated text, selected text, clipboard contents
and API keys are never included. **Copy diagnostics** puts a fuller text (the same information plus the
full redacted log, up to 200 KB) on the clipboard for you to review and paste; check it before you post.

Security problems go through [SECURITY.md](SECURITY.md), never a public issue.
