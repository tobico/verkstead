# 03. Busy shells

## What to build

A terminal tab's × asks before it ends a shell somebody is working in
(ADR-0019, *Tabs and groups*).

**Busy is the server's judgement, and it is read by name.** The pty the server
holds answers `tcgetpgrp` with the pid of whatever process group is in the
foreground, and busy is that pid running something other than the shell the
terminal was started with — on Linux, the name in `/proc/<pid>/comm` compared
against the shell's own basename. Comparing pids instead cannot be done: the
child the register's terminal was spawned as is the sandbox wrapper, so the
shell is a grandchild in a pid namespace of its own and behind the worktree's
dev shell where its flake has one, and the `setsid` in the terminal's spawn
lands on the wrapper, making the wrapper the pty's session leader too.

**This is proven rather than assumed.** A probe during planning — a pty, a
`bwrap --unshare-all` wrapper, `/bin/sh -i` inside — read the foreground group
as the shell at an idle prompt and as `sleep` while one ran, with both pids
visible in the server's own namespace and both names readable. The wrapper's
pid differed from the shell's, which is the whole of why the comparison is by
name.

**The pty has to reach the register for any of this to be readable.** The
register holds a Screen and the two channels that end a terminal and neither
the pty nor the child; the pty is already an `Arc` shared with the Screen, so
putting it on the register is a clone.

The terminals list and the close both report the flag, which changes the
terminals wire type — regenerate the viewer's types and the golden fixtures
from the real endpoints rather than editing either by hand. A × on a busy tab
asks first; an idle one does not. Where the platform cannot answer — the ConPTY
on Windows — the flag reads busy and every close confirms.

## Acceptance criteria

- [ ] A `sleep` running in a terminal makes its × ask before it ends the
      shell; an idle prompt does not — in a worktree whose flake has a dev
      shell as well as one without.
- [ ] The terminals list and the close both carry the flag, with the viewer's
      types and the fixtures regenerated rather than written.
- [ ] The Windows suite sees the flag read busy, so every close there
      confirms.
