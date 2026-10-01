# 05. The macOS and Windows readers

## What to build

Task 01 reads a terminal's ports on Linux and says, on the other two
platforms, that it cannot. This task makes the same reading on both, so a
Forward from a Mac or a Windows member works as a Linux one does.

**macOS.** The Sandbox is `sandbox-exec` with the network allowed, and the
terminal's wrapper leads its own process group with a keeper beside it; there
is no `/proc`. Walk the wrapper's descendants and read their listening TCP
sockets with `lsof`, filtered to listeners and to those pids, parsed by port
and address. The reader runs on the same tick and reports through the same
reading and Nudge.

**Windows.** A terminal is a ConPTY under a Job Object of its own, so the
Job's process list is the tree; the TCP table with owning pids, listeners
only, matched against it, is the reading. Proven under Wine in the Windows
suite, as the rest of the Windows terminal is.

A platform's reader that cannot tell — `lsof` missing, the Job unreadable —
reads nothing and says so once in the log, rather than reading busy or failing
the attach.

## Acceptance criteria

- [ ] On macOS a server in a terminal attached from another device is listed within two seconds and dropped within two of ending, by the same reading and Nudge as Linux.
- [ ] On Windows the same, matched against the Job Object's pids, proven in the Windows suite under Wine.
- [ ] A reader that cannot tell reads nothing, logs once, and leaves the terminal attachable.
