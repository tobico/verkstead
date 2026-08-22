# 03. Watching a live session

## What to build

A live session's Screen, watched from the workbench as it is drawn.

The server holds the screen of every running session and feeds it from the same
bytes the Capture is written from, so it is current rather than as far as the
store last got. The browser attaches over a websocket — the first bidirectional
transport in the codebase; SSE and refetch stay the freshness model for
everything else. A repaint of the current grid on connect, raw bytes relayed
after.

The browser sends its window size up and the latest one wins for everybody:
one screen however many devices are watching, and the size reaches the
session's own terminal so its interface redraws to fit.

Reachable for every live session from its Conversation, grilling included.
Watching commits the human to nothing — no Timeline Event, no move, and nothing
about the run changes because somebody looked. A watcher that disappears leaves
the session exactly as it was.

No auth of its own: the tailnet is the perimeter, as it is for every other
endpoint.

## Acceptance criteria

- [ ] Two browsers attached to one session see the same screen, and one that
      reconnects gets a repaint matching it.
- [ ] Resizing a watcher's window changes what the session's own interface
      draws.
- [ ] Attaching, watching and closing leave no Event on the Timeline and change
      nothing about what the run does.
- [ ] A session that has ended still shows its last screen, read-only, and
      refuses input.
