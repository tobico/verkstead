# 02. The desktop binary retired into the Windows shim

## What to build

`verkstead-desktop` stops being built on Linux and macOS — the desktop crate is
a library there. On Windows the binary remains, but as a thin windows-subsystem
shim: it starts `verkstead desktop` found beside its own image (never off the
PATH), forwards its arguments, and exits with the app's code — the
windows-subsystem attribute lives only here, so the unified binary stays an
ordinary console program everywhere.

The desktop test suite drives the unified binary through the verb, and the
packaging `.desktop` entry's `Exec` line says the verb too. The registration
variant task 01 kept alive for the old binary goes with the binary.

## Acceptance criteria

- [ ] No `verkstead-desktop` artifact is built on Linux or macOS.
- [ ] The Windows shim opens no console window, finds the CLI beside its own
      image, and its exit code is the app's.
- [ ] The desktop tests pass driving `verkstead desktop`.
- [ ] Every written autostart entry now carries the verb.
