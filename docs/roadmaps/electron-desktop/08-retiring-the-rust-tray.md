# 08. Retiring the Rust tray

## Goal

`verkstead` has no `desktop` verb and no `desktop` cargo feature;
`crates/desktop` is gone, with the shim, its build script, its suites and the
CLI's end-to-end desktop suite; CI installs no toolkit and runs no tray leg;
the flake's dev shell carries no GTK; the release workflow's closure checks
say what a slim binary must not link; the nix closure check still holds. The
documentation reads as if the tray app never shipped except where history
says it did.

## Decisions in force

- **Remove it whole** ([ADR-0020], Set 845 Q1 and Q1a): nothing is left that
  Electron cannot do. A crate kept for a fallback, or shrunk to a library,
  was rejected.
- **Only once no platform ships it** ([ROADMAP.md]): this stage needs 05, 06
  and 07 all landed.
- **The headless artifacts are unchanged**: they were built with the feature
  off, and now there is no feature to leave off. The `cargo tree` greps that
  policed GUI dependencies keep their job with the feature gone.
- **ADR-0012 is superseded**, its status line already written when the
  roadmap was committed; this stage is where what it describes stops
  existing.

## Proposed tasks (provisional)

1. **The crate and the verb** — `crates/desktop` out of the workspace, the
   feature and verb out of the CLI, the desktop test suite removed, the
   in-process `run_on_keyed` entry kept only if something still calls it.
   Accepts: `cargo build --workspace` links no toolkit; `verkstead --help`
   names no `desktop`.
2. **CI and the flake** — the toolkit install steps, the dbus tray leg and
   the shim feature gone from `ci.yml`; GTK and pkg-config out of the dev
   shell; the closure check updated. Accepts: CI green with fewer steps;
   `nix flake check` passes.
3. **The words** — `docs/releasing.md` rewritten for eight legs of the new
   shape; the desktop roadmap's index note updated to say what stands now;
   the CONTEXT.md sweep for `verkstead desktop`, the shim and the launcher.
   Accepts: a grep for `verkstead desktop` finds only ADR-0012 and the old
   roadmap's briefs.

## Re-verify at start

- All three platform stages landed and each Release leg is Electron's.
- Whatever stage 01 left in the server crate that only the tray app called.
- Whether the Windows session launcher or `session-account` verb touches
  anything in `crates/desktop`.

[ADR-0020]: ../../adr/0020-electron-desktop.md
[ROADMAP.md]: ROADMAP.md
