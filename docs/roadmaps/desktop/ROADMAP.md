# Desktop roadmap

Verkstead becomes a desktop app: a system-tray binary that runs the server
in-process and opens the embedded viewer in the default browser, shipped as a
Windows portable exe, a universal macOS dmg and a Linux x86_64 AppImage — while
the headless CLI goes on shipping exactly as it does today. The decisions and
their why are in [ADR-0012](../../adr/0012-desktop-tray-binary.md), which
revises [ADR-0004](../../adr/0004-single-binary-distribution.md) from one
binary to two single-file artifacts; the terms are in
[CONTEXT.md](../../../CONTEXT.md), which each stage updates as its piece lands.

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
- [ ] 05: Windows port and portable exe — [brief](05-windows-exe.md) *(in progress: `desktop/05-windows-exe`)*
