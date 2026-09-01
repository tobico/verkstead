# 01. The server compiles for Windows, and starts there

## What to build

The workspace builds for `x86_64-pc-windows-msvc` and its tests run there, and
`verkstead serve` on a stock Windows machine comes up with nothing set in the
environment and serves the viewer in a browser. Nothing about sessions is
solved here — that is the task after this one — but everything under them goes
on being compiled, because almost all of it already compiles anywhere.

**The gating is at the leaf call sites, not around the modules.** Measured
against the tree, the code that genuinely will not build on Windows is the
pseudo-terminal a session runs on, the exec-time calls the macOS keeper makes,
the process group a git fetch that ran past its deadline is killed by, and the
mode the secrets file is written with. `rustix` goes under a Unix-only
dependency table to match. Everything above them — the sessions module, the
runner, the Screen, the Capture, the transcript readers, the store, the render
crate and the CLI — is ordinary portable Rust and stays as it is. Keep the shape
the codebase already uses for this: a platform is a value both arms of which a
test on any machine can call, and a `cfg` only where the compiler leaves no
choice.

Two of those four are nothing to do with sessions and want a real Windows
answer rather than a gate. **A fetch that runs past its deadline is still
killed on Windows**, even where what that reaches is narrower than a process
group. And **the secrets file is either written no more readable than it is on
Unix, or the gap is said in the code where the mode is set** — a file holding a
token should not quietly become world-readable because the platform changed.

**`HOME` and the Build Cache resolve from the Windows environment.** Both are
refused at startup today and a stock Windows box sets neither, so the server
cannot start there at all. They resolve from `%USERPROFILE%` and
`%LOCALAPPDATA%` rather than being skipped on a platform that runs no sessions:
sessions on Windows are a later stage's, and both are what one will want when
it arrives. The Data Directory and the Log Directory already have their Windows
arms from stage 01 and need nothing. The Compile Server is started at a
session's launch rather than at the server's, so it needs no arm either.

**The Windows machine this project has is a CI job.** `cargo check --target`
from Linux is not it: MSVC needs a Windows host. A job beside `rust` and
`macos-sandbox` in `ci.yml`, pinned the way both of those are, is what says the
port holds — and what will say it goes on holding.

## Acceptance criteria

- [ ] A Windows job in `ci.yml` builds the workspace and runs `cargo test`, and
      the Linux and macOS jobs pass unchanged — both sandbox suites and the
      sessions suite included
- [ ] `verkstead serve` on a Windows machine with nothing set in the
      environment serves the viewer, and its startup line names a Data
      Directory under `%APPDATA%` and a Build Cache under `%LOCALAPPDATA%`
- [ ] A git fetch that runs past its deadline is killed on Windows, and the
      secrets file is written no more readable there than on Unix — or the
      code says plainly why it cannot be
- [ ] CONTEXT.md's Build Cache term says where the directory goes on Windows,
      as stage 01 amended the Data Directory's
