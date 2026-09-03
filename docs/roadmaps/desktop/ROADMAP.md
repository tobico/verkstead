# Desktop roadmap

Verkstead becomes a desktop app: a system-tray binary that runs the server
in-process and opens the embedded viewer in the default browser, shipped as a
Windows portable exe, a universal macOS dmg and a Linux x86_64 AppImage — while
the headless CLI goes on shipping exactly as it does today. The decisions and
their why are in [ADR-0012](../../adr/0012-desktop-tray-binary.md), which
revises [ADR-0004](../../adr/0004-single-binary-distribution.md) from one
binary to two single-file artifacts; the terms are in
[CONTEXT.md](../../../CONTEXT.md), which each stage updates as its piece lands.

**Finished, and since amended.** What this roadmap shipped as two binaries is
one again: the tray app is `verkstead desktop`, a default-on feature of the CLI
rather than a second binary, and the Windows download is an msi carrying that
binary and the shim a Start-menu shortcut names. ADR-0012's amendment says why,
and [adoption.md](../../adoption.md#getting-it-running) describes the artifacts
as they now are. The briefs below are left exactly as they were written: they
are the record of what each stage set out to build, rather than a description
of what stands today — so a `verkstead-desktop` in one of them is that stage's
own name for what is now a verb.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

Partly reorderable: 02 needs 01, and 03, 04 and 05 each need 02. 03 comes
first of the three, because it lays the shared release plumbing and the
packaging assets that 04 and 05 both build on. After that 04 and 05 run in
either order and there is no string between them: the "sessions need Linux" UI
state was to have landed in whichever ran first and been reused by the other,
and 04 ran and ported the Sandbox instead — so nothing of that state exists,
Windows is the only platform without sessions, and building it is 05's alone.

## Stages

- [x] 01: Platform directories — [brief](01-platform-directories.md)
- [x] 02: The desktop crate and the Linux tray — [brief](02-desktop-crate.md)
- [x] 03: AppImage and the release legs — [brief](03-appimage.md)
- [x] 04: macOS bundle and dmg — [brief](04-macos-dmg.md)
- [x] 05: Windows port and portable exe — [brief](05-windows-exe.md)
