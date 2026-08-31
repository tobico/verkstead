# 04. macOS bundle and dmg

## Goal

A `v*` tag builds a universal `Verkstead.app` inside a dmg; on a Mac the tray
runs, the viewer opens, and starting a session is met by an honest "sessions
need Linux" state in the UI rather than a broken spawn. The README documents
the unsigned-app approval steps.

## Decisions in force

- **Sessions stay Linux-only, said plainly** ([ADR-0012], grilling Q1): the
  Mac build is the full product minus running sessions, with the UI saying so
  where a session would start. Porting the sandbox is later work; remote-client
  apps were rejected. This stage builds that UI state — mac is the first
  non-Linux ship — and stage 05 reuses it.
- **Universal binary** via `lipo` over the two Apple targets the release
  already builds; bundle id `net.tobico.Verkstead` (Q4).
- **Unsigned** (Q9a): no Developer ID for now. Gatekeeper on current macOS
  sends users to System Settings → Privacy & Security to approve an unsigned
  app — document that dance rather than pay for signing. Revisit if the
  audience outgrows early adopters.
- **No double-click default action on macOS** — the tray icon opens its menu
  on click, which is the platform's way; "where possible" per the Brief.
- Launch on Startup uses a LaunchAgents plist (or the modern
  `SMAppService` equivalent), re-registered on every launch per Q6.

## Proposed tasks (provisional)

1. **The sessions-need-Linux state** — server compiled without session support
   reports it; the viewer draws the state where sessions would start. Accepts:
   vitest coverage of the state; a mac (or gated Linux) build answers the API
   honestly.
2. **Bundle and dmg** — scripted `.app` layout (Info.plist, icns from the
   hammer icon), `lipo`, dmg packing. Accepts: script runs on a macos runner;
   the app launches from the mounted dmg on a fresh runner.
3. **Release leg and docs** — the macOS desktop job joins `release.yml` on the
   plumbing stage 03 laid: a build leg and an artifact name, and nothing about
   `publish`. The README gains the unsigned-open steps. Accepts: tag run
   attaches the dmg.

## Re-verify at start

- What actually happens today when a session starts on a mac build (where the
  bwrap spawn fails) — the gate may want to sit further up than the spawn.
- The release matrix still builds `x86_64-apple-darwin` on `macos-15-intel`
  and aarch64 on `macos-15`; whether one runner can `lipo` both halves or the
  job needs both artifacts downloaded.
- What stage 02 shipped for the tray event loop — winit/tao on macOS must run
  on the main thread; confirm the crate boots that way on a mac at all before
  scripting the bundle.

[ADR-0012]: ../../adr/0012-desktop-tray-binary.md
