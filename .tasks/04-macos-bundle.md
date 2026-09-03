# 04. The macOS bundle

## What to build

The dmg carries the unified binary, still built for both architectures and
`lipo`d together. The bundle's executable — what `CFBundleExecutable` names —
becomes a tiny launcher script that execs `verkstead desktop`, since a bundle
cannot pass a verb to a bare executable and the app stays unsigned by
ADR-0012's standing decision.

## Acceptance criteria

- [ ] Double-clicking the app runs the tray app.
- [ ] The bundled binary answers `ask` and `guide`.
- [ ] The dmg leg of the release workflow builds and uploads it.
