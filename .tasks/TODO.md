# One verkstead binary

The sandbox hands every session the running server's own image, bound first on
the session's PATH so the two halves of an ask can never skew — and since the
server moved in-process into the desktop app, that image has been a tray app
with no `ask` in it: every spawned agent got a binary it could not ask with.
The grilling settled the fix as unification: one `verkstead` binary again, the
tray becoming a `desktop` verb behind a default-on cargo feature, the static
musl CLI still shipping with the feature off, thin shims for the two launchers
that cannot say a verb, an msi replacing the Windows portable exe, and the
server probing its own image at startup instead of trusting the invariant.
ADR-0012 is amended beside this plan.

## Tasks

- [x] 01: The desktop verb — [details](01-desktop-verb.md)
- [x] 02: The desktop binary retired into the Windows shim — [details](02-windows-shim.md)
- [x] 03: The Linux artifacts — [details](03-linux-artifacts.md)
- [x] 04: The macOS bundle — [details](04-macos-bundle.md)
- [x] 05: The Windows msi — [details](05-windows-msi.md)
- [x] 06: The startup probe — [details](06-startup-probe.md)
- [x] 07: The record — [details](07-the-record.md)
