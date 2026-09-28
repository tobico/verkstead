# Retiring the Rust tray

The Electron app ships on all three platforms, so the Rust tray app has no
release leg left to lose. This stage takes it away whole (ADR-0020): no
`desktop` verb, no `desktop` cargo feature, no `crates/desktop` — and with them
the Windows shim, its build script, its suites and the CLI's end-to-end desktop
suite. CI installs no toolkit and runs no tray leg, the development shell
carries no GTK, and the checks that kept a GUI out of the headless binary keep
that job in terms of what a slim binary must not link rather than of a feature
that was turned off.

What is left afterwards reads as if the tray app never shipped, except where
history says it did: ADR-0012 stands as the superseded record of what this
deletes, and the code that takes over the old app's launch agent and Run value
on the machines it ran on stays exactly where it is.

Roadmap stage: [08: Retiring the Rust tray](docs/roadmaps/electron-desktop/08-retiring-the-rust-tray.md)

## Tasks

- [x] 01: The verb and the crate — [details](01-the-verb-and-the-crate.md)
- [x] 02: The flake, and what a slim binary must not link — [details](02-the-flake-and-the-closure-checks.md)
- [x] 03: The words — [details](03-the-words.md)
