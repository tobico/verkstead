# Windows sessions roadmap

Windows runs sessions, sandboxed. The [desktop roadmap](../desktop/ROADMAP.md)
ported the product to Windows with the session machinery gated out at its leaf
call sites and one honest refusal where a session would start; this roadmap is
the stage that port's brief named as future work. The decisions and their why
are in [ADR-0014](../../adr/0014-windows-sessions.md), which is the sibling of
[ADR-0012](../../adr/0012-desktop-tray-binary.md)'s macOS amendment one
platform over; the terms are in [CONTEXT.md](../../../CONTEXT.md), which each
stage updates as its piece lands.

Two things stand between a Windows Verkstead and a session — the
pseudo-terminal and the Sandbox — and they land separately, in that order.
Between the two, a Windows session runs unsandboxed and the workbench says so;
that state is the first stage's to build and the third's to remove.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

Strictly ordered. 02 needs 01 only for the Windows end-to-end suite it proves
itself on; 03 needs both — the fresh profile and the terminal from 01, and the
pipe from 02, without which a container's session could not ask. 03 opens
with a probe the human runs on a Windows machine, and its own grilling reads
that output before the rendering is written.

## Stages

- [x] 01: The ConPTY terminal and unsandboxed sessions — [brief](01-conpty-terminal.md)
- [ ] 02: The named pipe — [brief](02-named-pipe.md) *(in progress: `windows-sessions/02-named-pipe`)*
- [ ] 03: The AppContainer — [brief](03-appcontainer.md)
