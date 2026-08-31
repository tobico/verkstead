# 05. Windows port and portable exe

## Goal

The workspace compiles for `x86_64-pc-windows-msvc` with the session machinery
cfg-gated out; `verkstead-desktop.exe` is a portable single file — tray with
double-click Open, server in-process, browser UI, sessions honestly absent via
stage 04's UI state — attached to releases by a Windows leg of the workflow.

## Decisions in force

- **Port by gating, not by porting sessions** ([ADR-0012], grilling Q1): the
  bubblewrap sandbox, the `rustix` PTY, the Screen and everything downstream of
  a running session are compiled out on Windows. A real Windows session story
  (ConPTY, a sandbox answer) is future work, not this stage's.
- **`HOME` is a hard startup requirement today**
  (`crates/server/src/lib.rs`) — on Windows the equivalent comes from the
  platform, alongside stage 01's directory logic.
- **Portable exe** per the Brief: no installer, no MSI; the exe runs from
  wherever it sits. Launch on Startup writes the registry Run key with the
  current exe path, re-registered every launch (Q6) — which is exactly the
  behaviour a moved portable exe needs.
- **Unsigned** (Q9b): SmartScreen's warning has a visible "run anyway" path
  and certs cost real money; ship unsigned for the early-adopter audience.
- The CLI's four release legs are untouched; whether a bare Windows *CLI*
  binary is worth attaching is this stage's call to put to the human, not a
  decision already made.

## Proposed tasks (provisional)

1. **Compile the server for Windows** — cfg-gate `sandbox`, `terminal`, the
   session spawn path and their dependents; replace `HOME` and any remaining
   Unix-isms in the kept code. Accepts: `cargo check --target
   x86_64-pc-windows-msvc` green in CI for the workspace minus gated modules;
   the sessions-need-Linux API state holds.
2. **The desktop binary on Windows** — tray with double-click Open, Run-key
   startup, dialog on taken port, logs under `%LOCALAPPDATA%`. Accepts: manual
   smoke on a Windows machine or runner screenshotting; unit tests where the
   tray libs allow.
3. **Release leg** — a `windows-latest` job builds the exe on the plumbing
   stage 03 laid: a build leg and an artifact name, and nothing about
   `publish` — with launch assertions of its own. Accepts: tag run attaches
   `verkstead-desktop-windows-x64.exe` (naming per the existing scheme, and per
   what stage 03 taught `publish` to carry).

## Re-verify at start

- How far the gating actually reaches — transcript readers, the Compile
  Server, `bwrap` call sites, `rustix` features — measured against the tree as
  it stands, not the list above.
- sqlx/SQLite, the push notifier and the embedded viewer on Windows: expected
  fine, unverified anywhere.
- Stage 04 landed the sessions-need-Linux state; if 05 runs first, that state
  moves here.

[ADR-0012]: ../../adr/0012-desktop-tray-binary.md
