# Security Policy

This is the full security policy for the source repository. The short public version that ships with the
binaries is [docs/public/SECURITY.md](docs/public/SECURITY.md).

## Supported versions

Security fixes are released for the latest published version. Older releases are not patched; upgrade to the
latest release. The version and date of each release are in [docs/public/CHANGELOG.md](docs/public/CHANGELOG.md).

## Reporting a vulnerability

Report privately to **versotranslate@gmail.com**. Do not open a public issue, and do not include a working exploit in
a pull request.

Please include:

- the version (tray menu › About Verso) and the OS and version;
- a description of the issue and its impact;
- steps to reproduce, or a minimal proof of concept;
- relevant log lines (see `TROUBLESHOOTING.md` for their location);
- how you want to be credited.

We aim to acknowledge within three business days, to confirm scope within ten, and to agree a disclosure
date with you. Please give us a reasonable window to ship a fix before publishing.

## Scope

In scope:

- the Verso application and its installers;
- the release pipeline (build, signing when enabled, publication to the public repository);
- the documents and scripts in this repository that affect security.

Out of scope:

- the third-party translation services (report those to their operators);
- problems that need an already-compromised machine or a modified build;
- denial of service against third-party services, or testing against systems you do not own.

## Safe harbour

We will not pursue legal action against researchers who follow this policy, act in good faith, avoid privacy
violations, avoid destroying data, and give us time to fix the issue before disclosure.

## Secrets

The project's credentials live only in the CI secret store. No secret belongs in the repository, in logs, in
artefacts or in documents. If you find one exposed, report it privately at once; do not use it.
