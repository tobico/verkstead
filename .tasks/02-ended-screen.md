# 02. The Screen of a session that has ended

## What to build

The Screen of a session that is over: what its terminal last showed, drawn as a
terminal rather than as bytes.

The server holds a virtual terminal and builds the screen by replaying the
session's Capture through it — the Capture is the record, and the Screen is
what that record leaves on a terminal. It hands the current grid back as the
escape sequences that would paint it, which is what the browser draws and, in
task 03, what a fresh attach repaints from. Grid only: no scrollback is kept,
however much the session printed.

In the workbench the session's details pane grows a two-way switch at the top —
Transcript and Screen — opening on the Transcript, which is what a reader
usually came for. The Screen draws in xterm.js. That is the browser's window
onto the server's grid and the one deliberate exception to the rule that the
browser never parses, argued in [ADR
0007](docs/adr/0007-server-held-terminal.md); the server's virtual terminal
stays the source of truth.

Read-only. An ended session's Screen is the last it stood and there is nowhere
to type; a live session's repaints from wherever its Capture has got to, and
making that continuous is task 03's.

## Acceptance criteria

- [ ] Opening a session that has ended and switching to Screen shows the last
      screen it left, drawn as a terminal.
- [ ] The grid the server hands back for a fixture stream is what a real
      terminal would show for those bytes, including a session that ended on
      the alternate screen.
- [ ] Nothing above the top of the grid is kept, whatever the session printed.
- [ ] The pane says it is read-only and takes no input.
