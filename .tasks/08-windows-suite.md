# 08. The Windows end-to-end suite

## What to build

A `cfg(windows)` sessions suite — a dozen or so tests — that proves a Windows
session end to end the way the Unix one does, with a PowerShell script standing
where the agent goes. The Unix sessions suite stays `cfg(unix)` and is not made
portable: it stands on a mount namespace and a `/bin/sh` probe, and what is
being proved here is a different machine.

**Everything real except the agent**: a real repository, a worktree git made,
the open rendering, a real ConPTY, the fresh profile, the prompt file. The
stand-in is handed exactly what the backend it stands where would be, and on
Windows it reads its Brief out of the file named on its one line rather than off
its command line.

What the suite asks, in the Unix suite's own terms: a session starts; what it
prints reaches the Capture; a resize from a watcher reaches the process; typing
reaches it; ending it ends it; the prompt file holds the Brief; the fresh
profile's variables resolve where they should; the unsandboxed value is on the
Conversation view; a Conversation Terminal opens on `pwsh`.

**CI needs almost nothing.** The `windows-2025` job already builds the whole
workspace with `--all-targets` and runs `cargo test --workspace`, so a
`cfg(windows)` file runs there with no workflow change. What it does need is
**sccache installed on the runner**, which is what makes the Compile Server case
real: the Compile Server is the one thing the open rendering runs that is not a
session, and it only starts where the server can see an sccache.

## Acceptance criteria

- [ ] The `windows-2025` job runs the suite green: start, output reaching the
      Capture, a resize reaching the process, typing, ending, the prompt file,
      the fresh profile's variables, the note on the view, a terminal on `pwsh`.
- [ ] A session's Capture holds what the stand-in printed, read back off the
      Timeline the way the Unix suite reads one.
- [ ] With sccache on the runner, the Compile Server comes up as a plain process
      through the open rendering, and a session's environment names it.
