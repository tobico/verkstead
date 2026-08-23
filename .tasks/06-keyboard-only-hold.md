# 06. Keyboard-only Hold, mouse flows free

## What to build

Stop the mouse taking the Hold. When a session's TUI turns on mouse tracking,
the browser terminal reports every movement over it down the same data path
as keystrokes, the socket sends each report as typed input, and the server
takes the Hold on the first byte — so glancing a cursor across a live Screen
silently stops Verkstead ending sessions.

The settled rule: **only the keyboard — paste included — takes the Hold.
Mouse input always flows to the session, held or not, and never takes it.**
The Hold's one job is preventing auto-shutdown while a human is deliberately
intervening; a mouse report is not that.

Two halves:

- **The socket grows a second input kind** beside the existing typed one: the
  same bytes on their way to the session's terminal, but written through
  without touching the Hold. The typed kind keeps taking the Hold exactly as
  it does now.
- **The browser tells the two apart at the source.** The terminal emits
  keyboard input, paste, and mouse reports through one data callback, so the
  split is made by what the human actually did: input on the heels of a key
  or paste event goes as typed, everything else — mouse reports, and
  whatever the terminal synthesises from the wheel — goes as the new kind.

The Hold's meaning has narrowed, and the words around it follow: the Screen's
"type to take the keyboard" note stays true, but the module documentation and
`CONTEXT.md`'s account of the Hold say that mouse input flows without taking
it and the Hold exists only to stop auto-shutdown.

## Acceptance criteria

- [ ] Moving, clicking or scrolling the mouse over a live unheld Screen never
      takes the Hold, and the session's TUI still receives and reacts to the
      mouse input.
- [ ] The first keystroke or paste takes the Hold exactly as before, and
      mouse input keeps flowing while it is held.
- [ ] A server test covers the new input kind writing through without a Hold
      appearing; a web test covers the browser's split of key, paste and
      mouse input.
- [ ] The Hold documentation says what the Hold now means.
