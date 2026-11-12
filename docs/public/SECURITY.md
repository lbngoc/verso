# Security

## Reporting a problem

Please report security problems **privately**, by email to **versotranslate@gmail.com**. Do not open a public issue
for a vulnerability.

Include:

- the Verso version (tray menu › About Verso) and your OS version;
- what you did, and what happened;
- any log lines or files that help (see [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for their location);
- how you would like to be credited, if you want to be.

We aim to acknowledge a report within three business days and to say whether it is in scope and when a fix
is expected. Please give us a reasonable window to release a fix before writing about it publicly.

## Scope

- The Verso application and its installers published in this repository.
- The release pipeline that builds and publishes those installers.

The translation services Verso talks to (Google Translate, Microsoft Translator) are out of scope; report
problems with them to their operators. Please do not test against third-party services or other people's
devices.

## What we will not ask for

We never ask for your account password, and Verso never asks for credentials: it has no account, no API key
and no telemetry. Treat any message that asks for one as fraudulent.

## Supported versions

Security fixes are released for the latest published version. The version and date are in
[CHANGELOG.md](CHANGELOG.md).
