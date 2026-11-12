# Contributing to Verso

Thanks for wanting to help. The source is open under the MIT licence; issues and pull requests are welcome.

## Before you start

- Read the [code of conduct](CODE_OF_CONDUCT.md) and the [security policy](SECURITY.md). Do not open a
  public issue for a security problem.
- For a bug, open an issue with the version, OS, steps and log excerpt. For a feature, open an issue first
  so we can agree on the shape before code is written.
- One change per pull request; small, focused pull requests are reviewed faster.

## Setting up

Requirements: Node 20+, Rust 1.80+ with `rustfmt` and `clippy`, and the platform toolchain (Xcode command
line tools on macOS, Visual Studio Build Tools with the C++ workload on Windows).

```sh
npm ci
npm run tauri dev          # run the app with the Vite dev server
npm run check              # svelte-check
npm test                   # Vitest
npm run lint               # ESLint
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml --check
```

Build installers with `npm run tauri build` (macOS) or `npm run build:windows` (Windows cross-build from
macOS; native Windows uses `npm run tauri build`).

The project keeps its planning notes outside this repository. A pull request should describe the problem,
the change and how it was tested; link an issue when there is one.

## Style

- TypeScript and Svelte: Prettier (`npm run format`) and ESLint; no `any` without a reason.
- Rust: `rustfmt` defaults, clippy clean, errors as `thiserror`/strings at the command boundary.
- User-visible strings go through `src/lib/i18n/en.json` and `vi.json`; both files need the same keys.
- Commits: a short imperative subject, then why in the body. Do not commit build output, logs or secrets.

## Pull requests

- Describe the behaviour before and after, and how you tested it (commands, OS, VM).
- Add or update tests for logic changes; a UI-only change needs a screenshot.
- By opening a pull request you agree that your contribution is licensed under the MIT licence.

## Questions

Open an issue with the `question` label.
