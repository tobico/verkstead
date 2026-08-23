# Agent output polish

Six settled changes to the agent-output Timeline item and its details pane.
The line count becomes a turn count read off the Transcript; a spinner and an
empty circle replace the "running" text everywhere a session's liveliness is
said; the Transcript/Screen switcher moves into the header and its active
indicator slides; the Screen fills the pane's height as it already fills its
width; and the Hold is taken by the keyboard alone, with mouse input flowing
to the session without taking it.

Settled in the grilling on this Conversation: the metric counts turns as the
Transcript pane draws them (no bookkeeping), idle is the session's quiet
clock past 3 seconds, the slide is ease-in-out at 100ms, rows follow the
latest window the way columns do, and the Hold exists only to prevent
auto-shutdown.

## Tasks

- [x] 01: Turns for lines — [details](01-turns-for-lines.md)
- [x] 02: The idle mark — [details](02-idle-mark.md)
- [x] 03: The idle mark on the sidebar card — [details](03-idle-mark-on-card.md)
- [x] 04: The switcher into the header — [details](04-switcher-into-header.md)
- [ ] 05: The Screen fills the pane — [details](05-screen-fills-pane.md)
- [ ] 06: Keyboard-only Hold, mouse flows free — [details](06-keyboard-only-hold.md)
