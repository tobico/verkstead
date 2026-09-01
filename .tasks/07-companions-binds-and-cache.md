# 07. Companions, configured binds and the build cache

## What to build

The rest of the Sandbox's surface on macOS, mirroring what the Linux suite
already proves about each piece.

**Companion Repos**, each at the mode the human set it to: its worktree and its
git directory together, read-only or read-write, and a read-only companion's own
configured binds still writable where the Linux surface makes them so.

**The Sandbox Configuration's entries**, the installation's and the settings
file's composing the way they do on Linux — a bare path reaching every session, a
named one reaching only sessions working in the Repo registered under that name,
and an entry naming something that is not there skipped rather than refusing the
session.

**The shared Build Cache**, writable, with the session's cargo home inside it and
the sccache the machine compiles through reachable — and the **Compile Server**,
which is Verkstead's own sccache running in a Sandbox of its own holding the
Worktrees directory and the cache and nothing else Verkstead keeps. Both need
their macOS surface, because the compile server's sandbox is composed by the same
code the sessions' is.

Everything here is proved by probes inside a real sandbox on a Mac, the way the
task before it was and the way the Linux suite is.

## Acceptance criteria

- [ ] Probes show each Companion Repo at the mode it was set to, and the
      configured binds composing as they do on Linux
- [ ] A probe shows the Build Cache writable with cargo's home inside it, and a
      Rust build inside a session compiling through the machine's one sccache
- [ ] A probe shows the Compile Server's own sandbox holding the Worktrees and
      none of the Data Directory
