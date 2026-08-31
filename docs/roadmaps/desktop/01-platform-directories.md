# 01. Platform directories

## Goal

`verkstead serve` started with no `--data-dir` puts everything in the platform
data dir — `~/.local/share/verkstead` on Linux, `~/Library/Application
Support/Verkstead` on macOS, `%APPDATA%\Verkstead` on Windows — instead of the
working directory, on every entry point alike; and a state/log-dir helper
stands ready for the desktop binary's log file. The dev docs show the old
behaviour spelled explicitly (`--data-dir .`).

## Decisions in force

- **The single Data Directory model stays** ([ADR-0012], grilling Q7): one
  directory holds the database, worktrees, skills, handoffs and both settings
  files, exactly as [CONTEXT.md](../../../CONTEXT.md) defines the term. Only
  its *default* moves. A full config/data/state split was considered and
  rejected as churn through settings, worktrees and handoffs for no user-visible
  gain.
- **The new default applies everywhere** (Q7a) — `verkstead serve` as much as
  the desktop binary. One rule; developers pass `--data-dir .`. Breaking
  nobody: nothing has ever been released (`nix/release.json` is 0.0.0) and the
  NixOS module passes `--data-dir` explicitly, so no migration code is written.
- **The build cache is already right** (`$XDG_CACHE_HOME/verkstead` or
  `~/.cache/verkstead`, `crates/server/src/build_cache.rs`) and does not move.
- **The state/log dir** is `~/.local/state/verkstead` on Linux,
  `~/Library/Logs/Verkstead` on macOS, `%LOCALAPPDATA%\Verkstead` on Windows —
  used by stage 02 for the desktop log file; the server itself keeps logging to
  stdout.
- Whether to hand-roll the three platforms the way `build_cache.rs` hand-rolls
  XDG, or adopt the `directories` crate, is the stage's own call — weigh one
  new dependency against a third copy of env-var logic.

## Proposed tasks (provisional)

1. **Platform data-dir default** — resolution replaces the `.` default on the
   `--data-dir` flag; the startup log line keeps saying which directory was
   chosen. Accepts: with the env unset the resolved path is the platform dir on
   each OS (unit-tested via env manipulation); an explicit `--data-dir` wins
   unchanged.
2. **State/log dir helper** — resolution beside the data-dir's, exported for
   stage 02. Accepts: per-OS unit tests; nothing writes there yet.
3. **Docs** — `docs/development.md` gains `--data-dir .` in the dev commands;
   `docs/adoption.md` names the new default.

## Re-verify at start

- `build_cache.rs::default_dir` is still the only XDG code in the tree, and
  the `--data-dir` default is still `.` in `crates/server/src/lib.rs`.
- The NixOS module still passes `--data-dir` explicitly (`nix/module.nix`).
- Windows is not yet compiled in CI — the Windows arm of the resolution can
  only be unit-tested cross-platform-shaped until stage 05; keep it free of
  Windows-only APIs.

[ADR-0012]: ../../adr/0012-desktop-tray-binary.md
