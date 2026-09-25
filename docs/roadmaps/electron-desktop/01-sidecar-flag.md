# 01. The sidecar flag

## Goal

`verkstead serve --desktop` runs the server exactly as `verkstead serve` does,
with two differences a desktop app needs: its startup line names the address
and no key, and a refused `tailscale serve` press raises the platform's own
password dialog instead of handing back the `sudo` line. Rust only, nothing of
Electron in it, and the Rust tray app goes on working — through the moved
module — so the stage ships alone.

## Decisions in force

- **The sidecar is the CLI binary, told by a flag** ([ADR-0020], Set 845 Q1
  and Q2): the Electron app starts the headless `verkstead` with `--desktop`.
  An environment variable and a plain `serve` with Electron filtering the log
  were both rejected — the first for being the flag by another door, the
  second for putting a regex between a secret and a file on a desk.
- **What the flag changes is behaviour, never the wire** (Q2, Set 847 Q12).
  Nothing about it reaches `AppState`'s serialisable surface or the viewer;
  the viewer learns it is inside the app from the preload bridge in stage 03.
- **The key stays off the desktop's startup line** ([ADR-0015] as amended):
  the `HandsOverTheLink::TheCaller` arm the tray app passes in-process is what
  the flag selects. `verkstead serve` without it is unchanged and keeps the
  link in its line.
- **The graphical grant moves into the server crate as it is** (Q2). Its three
  arms — `pkexec`, `osascript` *with administrator privileges*, PowerShell's
  `Start-Process -Verb RunAs` — are spawned commands with no toolkit behind
  them, and the arm is a value rather than a `cfg` so every machine's tests
  build every arm. The flag installs it where there is a screen, as the tray
  app does today; the `Elevate` seam and `Raised` stay what they are.
- **The screen probe moves with it**: `$DISPLAY` or `$WAYLAND_DISPLAY` on
  Linux, always on a Mac, the visible window station on Windows.

## Proposed tasks (provisional)

1. **The flag, and the line without the key** — `--desktop` on the server's
   `Config`, selecting the caller-hands-over arm. Accepts: `serve --desktop`
   logs the address alone; `serve` still logs the link; a run with the flag
   serves the workbench and answers `ask` from the same binary.
2. **The grant and the screen, in the server crate** — the module and its
   tests moved out of `crates/desktop`, the tray app pointed at the moved one.
   Accepts: the desktop crate's suite passes unchanged; the server crate's
   tests build all three arms on Linux.
3. **The flag turns the grant on** — a refused serve press under `--desktop`
   raises the platform's asking where there is a screen, and shows the line
   where there is none. Accepts: the remote suite's stand-in `tailscale` sees
   the elevated re-try under the flag and the bare refusal without it; the
   onboarding install's one elevated command goes the same way.
4. **Help text and the CLI suite** — the flag documented as the desktop app's
   and nobody else's. Accepts: `verkstead serve --help` names it; CONTEXT.md's
   Workbench Key entry says which install redacts and why.

## Re-verify at start

- `run_on_keyed` and `HandsOverTheLink` still exist in the server's `lib.rs`
  and `key.rs` in the shapes the tray app calls them with.
- `elevate.rs` and `screen.rs` in `crates/desktop` still have no toolkit
  dependency — the whole reason they can move.
- The onboarding wizard's install run still takes the same `Elevate` handle
  the Remote access pane does.

[ADR-0020]: ../../adr/0020-electron-desktop.md
[ADR-0015]: ../../adr/0015-open-boundary-and-workbench-key.md
