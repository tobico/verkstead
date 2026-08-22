# 01. Verkstead owns the terminal

## What to build

Verkstead allocates each session's pseudo-terminal itself and starts the
sandbox with the slave as its stdin, stdout and stderr, dropping the `script`
wrapper the session used to run under. The relay reads the master instead of a
pipe.

The terminal is made 100 columns by 30 rows, and there is a way to resize it
afterwards that the running session sees — that is what a watcher's window will
drive in task 03. It is told it is `xterm-256color`, which nothing set before:
the sandbox exports four variables and no terminal, and what a session's
interface draws depends on which one it thinks it has.

The session's own errors and the sandbox's own complaints now arrive together
on the one terminal, which is what a real terminal does. The separate error
pipe goes, and with it the `[the sandbox, not the agent]` marker: a sandbox
that refuses to start says so in the Capture of the session that failed, where
it happened, rather than appended after the session ended.

Everything downstream of the relay is unchanged. What a session prints reaches
the Capture and its Timeline row on the same cadence, the quiet clock still
moves on output alone, the transcript is still tailed beside it, and a session
is still ended by killing the sandbox around it.

The master is writable and nothing writes it yet. Typing into it is task 04's.

## Acceptance criteria

- [ ] The session suite passes with `script` gone from what is spawned, and a
      session's output reaches its Capture and its Timeline row as before.
- [ ] A stub agent reads back 100 by 30 and `xterm-256color` from inside the
      sandbox, and resizing the terminal mid-session changes what it reads.
- [ ] A sandbox that cannot start has its complaint in the Capture of the
      session that failed.
- [ ] Nothing writes the master.
