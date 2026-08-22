# The Screen and the Hold

Any live session can be watched from the workbench as a real terminal — the
Screen — and typed into. The first keystroke takes a Hold: Verkstead goes on
recording but stops ending sessions and advancing runs until the human hands
back, and hand-back judges whatever they left by the ordinary end-of-session
rules. A session that has ended shows its last screen, read-only.

Verkstead owns the pseudo-terminal to do it, replacing the `script` wrapper
whose stdin was `/dev/null`. A server-side virtual terminal holds the
authoritative grid, fed from the same bytes the Capture is written from, and
the browser attaches over a websocket with xterm.js as the window — the one
deliberate exception to the browser never parsing, argued in
[ADR 0007](docs/adr/0007-server-held-terminal.md).

Roadmap stage: [02: The Screen and the Hold](docs/roadmaps/session-output/02-screen-and-hold.md)

## Tasks

- [x] 01: Verkstead owns the terminal — [details](01-owned-terminal.md)
- [x] 02: The Screen of a session that has ended — [details](02-ended-screen.md)
- [x] 03: Watching a live session — [details](03-watching-live.md)
- [x] 04: The Hold — [details](04-the-hold.md)
- [ ] 05: A Hold nobody came back to — [details](05-holding-push.md)
