# 02. The grant and the screen probe, in the server crate

## What to build

The graphical grant and the screen probe move out of `crates/desktop` into the
server crate, unchanged in behaviour, with their tests. Nothing new is asked of
either: this is the move that lets task 03 turn the grant on from a flag, and it
ships on its own because the tray app going on working through the moved ones is
what proves it.

**Why they can move at all** is that neither depends on the toolkit. The grant's
three arms — `pkexec`, `osascript` running the command *with administrator
privileges*, and PowerShell's `Start-Process -Verb RunAs` — are spawned commands
and nothing more, and the probe reads `$DISPLAY` or `$WAYLAND_DISPLAY` on Linux,
answers yes on a Mac, and asks Win32 whether this process's window station is one
somebody can see. Both already depend on the server crate and on nothing else
(ADR-0020), which is the whole reason the move is a move rather than a rewrite.

**Which arm the grant takes stays a value rather than a `cfg`**, and that is the
reason its tests build all three on every machine: the arm a Linux runner will
never run is still an arm it can build the command of and assert the quoting of.
The probe's arms stay `cfg`-gated as they are — a window station is a Windows
question, and there is no value that makes it a Linux one.

**The probe cannot keep its name.** `screen` is spoken for in the server crate: it
is the terminal grid the workbench draws, and **Screen** is that in CONTEXT.md's
glossary, so a second `screen` there would misname one of the two whichever way it
was read. It arrives as `display` instead, asked as `display::there_is_one()`,
beside the grant's own module rather than under it — the tray and the dialogs ask
this question to decide whether to *draw*, not whether to elevate. Both modules
are public now, because the tray app calls them from outside.

**Three callers move with it, and they are what proves the move**: the tray app's
choice of whether to install the grant at all, its choice of whether to raise an
icon, and the dialogs' own check that there is somewhere to draw before they draw
one. The `Elevate` seam and `Raised` stay exactly what they are.

Nothing in either crate needs a new dependency — the Win32 feature the probe's
call comes from is already among the server crate's, and the desktop crate's own
feature list is worth re-reading afterwards for one that is now nobody's.

## Acceptance criteria

- [ ] The desktop crate's suite passes unchanged, and nothing under
      `crates/desktop` builds a `pkexec`, `osascript` or `RunAs` command any more.
- [ ] The server crate's own tests build and assert all three arms of the grant on
      Linux, the quoting each of the two string-building arms does included.
- [ ] The tray app's grant, its icon raising and both dialogs ask the moved probe,
      under a name nothing reads as the **Screen**.
