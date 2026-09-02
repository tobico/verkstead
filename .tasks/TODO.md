# 05. Windows port and portable exe

The workspace compiles and runs on Windows, and `verkstead-desktop.exe` is a
portable single file — the tray with double-click Open, the server in-process,
the viewer in the browser — attached to releases by a Windows leg of the
workflow. Sessions are the one thing a Windows Verkstead has not got, and it
says so where a session would start rather than failing to spawn one.

The gating is narrower than the brief expected. Outside the pseudo-terminal,
what will not compile on Windows is four leaf call sites, so the session
machinery goes on being built there and one refusal above the spawn is what
stands in its way. `HOME` and the Build Cache resolve from the Windows
environment rather than being skipped, because sessions on Windows are a later
stage's and both are what one will want. The bare CLI joins the release matrix
as a fifth binary beside the exe.

Roadmap stage: [05: Windows port and portable exe](docs/roadmaps/desktop/05-windows-exe.md)

## Tasks

- [x] 01: The server compiles for Windows, and starts there — [details](01-server-on-windows.md)
- [x] 02: Sessions are honestly absent on Windows — [details](02-sessions-absent-on-windows.md)
- [x] 03: The desktop app on Windows — [details](03-the-app-on-windows.md)
- [x] 04: The release legs, and the way in for a Windows reader — [details](04-release-legs-and-the-way-in.md)
