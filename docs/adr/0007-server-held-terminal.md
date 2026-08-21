# Server-held terminal, attached from the browser

Verkstead allocates and owns each session's PTY (replacing the `script`
wrapper, whose stdin was `/dev/null`) and holds the authoritative screen state
in a server-side virtual terminal fed from the Capture. The browser attaches
tmux-style over a websocket: a repaint of the current grid on connect, raw
bytes relayed after, keystrokes and resizes sent up, xterm.js as the window.
Every attached client sees the same screen; the latest resize sets the PTY
size.

This is the one deliberate exception to the rule that the browser never
parses (ADR 0003 direction, restated in the Capture viewer): a live
terminal is a terminal, not a document, and shipping server-rendered grids
per frame buys latency and wire cost for no fidelity. The exception is
bounded — the server's virtual terminal remains the source of truth (it is
what a fresh attach repaints from, and what a dead session's last screen is
read from), and the browser's copy is only a window onto it. Rendered
documents — Transcript, diffs, markdown — stay server-rendered as before.
