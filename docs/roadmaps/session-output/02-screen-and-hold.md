# 02. The Screen and the Hold

## Goal

Any live session can be watched from the workbench as a real terminal — the
Screen — and typed into: the first keystroke takes a Hold, Verkstead stops
ending or advancing anything until the human hands back, and hand-back judges
whatever they left by the ordinary end-of-session rules. A session that has
ended shows its last screen, read-only.

## Decisions in force

- **Server-held terminal, tmux-style attach** — [ADR
  0007](../../adr/0007-server-held-terminal.md): Verkstead owns the PTY
  (replacing `script`, whose stdin was `/dev/null`), a server-side virtual
  terminal (avt was the pick) holds the authoritative grid fed from the
  Capture stream, and the browser attaches over a websocket with xterm.js as
  the window — repaint on connect, raw bytes relayed after. The bounded
  exception to "the browser never parses" is argued in the ADR.
- **Screen and Hold are glossary terms** ([CONTEXT.md](../../../CONTEXT.md))
  and the behavioral decisions live there: grid only, no scrollback; one
  screen for every watcher, latest resize wins; watching commits to nothing;
  the Hold starts at the first keystroke and ends **only** by explicit
  hand-back — no timeout, no release on socket drop, because resuming over a
  half-finished intervention is worse than a stalled run.
- **Every live session is attachable**, grilling included — the hold only
  bites where Verkstead would otherwise end or advance something.
- **During a Hold Verkstead records and nothing else**: capture, tailing and
  Timeline updates continue; ending the session and advancing the run are
  suspended. A session that exits while held waits for hand-back; hand-back
  runs the normal evaluation (Step commit landed → on; otherwise →
  Interruption).
- **A held Conversation carries *blocked on you*** and can web-push after a
  while; **holds leave no Timeline Events** — the Timeline records the work,
  not the watching (the carve-out is written into the Timeline's glossary
  entry).
- **The *Take over manually* Remedy stays separate**: it remains "Verkstead
  steps aside, human uses a real terminal"; attach is for live sessions.
- **No new auth**: the tailnet is the perimeter, as for every existing
  endpoint (`crates/server/src/lib.rs` states the model). The websocket is
  the first bidirectional transport in the codebase; SSE + refetch stays the
  freshness model for everything else.
- Quiet-detection stays keyed on PTY output; keystrokes don't feed it — the
  Hold suspending session-end is what protects a human mid-typing.
- Default PTY size at spawn is fixed (e.g. 100×30) until a client resizes.

## Proposed tasks (provisional)

1. **Verkstead-owned PTY** — allocate the pty pair, hand the slave to bwrap
   as the session's stdio, drop `script`; relay reads the master; add
   resize. AC: sessions run and capture exactly as before (stub-agent suite
   green); a resize call changes what the TUI sees; stdin is writable but
   nothing writes it yet.
2. **Server-side Screen** — feed the VT from the capture stream (live) and
   by replay (ended sessions); expose repaint-as-escapes. AC: repaint of a
   live session matches what a real terminal would show for the fixture
   stream; an ended session yields its final grid.
3. **Watch-only attach** — websocket endpoint + xterm.js pane: repaint on
   connect, live bytes after, resize up (latest wins), read-only marker on
   ended sessions, reachable for every live session from the Conversation.
   AC: two clients see the same screen; reconnect repaints correctly; dead
   session shows last screen and refuses input.
4. **The Hold** — keystrokes to the PTY master; first keystroke flips the
   hold, badge + pending + push wired; hand-back control releases it and
   runs end-of-session evaluation; runner and quiet-end suspended while
   held. AC: a held Step session is never ended by quiet; a session that
   exits held advances nothing until hand-back, then evaluates normally;
   socket drop leaves the hold in place.

## Re-verify at start

- Stage 01 landed: capture tables/endpoint carry the new names, and the
  details pane is the Transcript (the Screen sits beside it, not instead).
- avt's current API still offers what ADR 0007 assumes (feed, resize,
  dump-current-state-as-escapes, alt screen); if not, revisit engine choice
  before building on it.
- `crates/server/tests/sessions.rs` exercises real `script` + bwrap —
  task 1 rewrites those assumptions; check what they've become.
- bwrap sandbox env sets no `TERM`/`COLUMNS`/`LINES` — decide what the
  owned PTY should export before the TUI's first paint.
- axum's websocket support in the pinned version; whether this stage's
  branch must stack on stage 01's unmerged PR (per
  `docs/agents/git-workflow.md`).
