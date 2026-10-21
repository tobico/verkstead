# 01. A retyped Preview starts from the worktree root

## What to build

A press on a Preview whose newest live terminal has the shell in front
*retypes* the command into that shell (ADR-0024, *Typed into a terminal, as
the human would have*). That is the one path that breaks today: the previous
run may have left the shell somewhere other than the worktree root — `cd web
&& pnpm dev` leaves it in `web/` — and the command typed again fails on its
own `cd`.

So, on a retype and only on a retype, the server reads the shell's working
directory and, where it is not the worktree root, types a line of its own
first:

    cd '<worktree root>'

followed by a return, and then the command as today. The human sees both
lines, in that order, as they would have typed them. Where the shell is still
at the root the command is typed alone, as today. Where the directory cannot
be read at all — a pid that has gone between the reading and the typing, a
`/proc` that will not answer — the `cd` line is typed anyway: it is harmless
at the root, and the other mistake is the one this task exists to stop.

**Reading the directory.** The press is decided against the foreground pid
the server already reads the shell's *name* off (the busy reading): on Linux
that pid's working directory is a link under `/proc`, and on macOS it is what
`proc_pidinfo` answers for the process's vnode path info. Those are exactly
the two platforms that ever retype — Windows refocuses a live terminal and
never types into it again — so no third platform needs a reading. The
worktree is bound inside the Sandbox at its own path, so what is read
compares against the Conversation's worktree path as the record holds it,
with no translation. Compare paths as the kernel reports them, not as
strings somebody normalised; a trailing separator is not a move.

**Quoting.** The root goes in single quotes, which bash, zsh, fish and
PowerShell all read as a literal; a path holding a single quote is not
handled, and need not be.

**The run's bookkeeping is unchanged.** The retype already marks the run as
typed before the line is written, so a second press in the moment between
finds the run on its way; the `cd` is a shell builtin, so no process group
of its own comes and goes, and the reading that follows sees the command's
groups as it does today.

**First presses, refocuses and new terminals are untouched.** A terminal
opened for a Preview starts at the worktree root already, so the `cd` would
be noise there.

**And the agent is asked to help.** The up arrow in that shell still replays
the command as it was written, so the Guide's *Suggesting a preview* section
and the shell line at the end of every building session's prompt (*A
terminal here runs …*) each gain one sentence asking for commands that leave
the shell where they found it — `pnpm --dir web dev`, `cargo run
--manifest-path`, a `-C` — over a `cd` joined by `&&`. One sentence each, in
the Guide's voice; the Guide's own tests cover its text.

The decision is recorded already, as the amendment at the end of ADR-0024
and in CONTEXT.md under **Terminal** (*Opened for a Preview*): build to
those words.

## Acceptance criteria

- [ ] On Linux and macOS, a Preview whose command moves the shell runs correctly on a second press: the terminal shows a `cd '<root>'` line and then the command, and the command runs at the root.
- [ ] A retype into a shell still at the worktree root types the command alone; a first press, a refocus and a terminal opened for something else in front behave exactly as before.
- [ ] A directory that cannot be read on a retype gets the `cd` line anyway, and nothing is typed on Windows that was not typed before.
- [ ] The sessions suite covers a retype after a run that moved the shell and a retype after one that did not.
- [ ] The Guide's preview section and the building session's prompt each carry the one-sentence request for commands that stay put, and the Guide's tests pass.
