# 01. Turns for lines

## What to build

Replace the "N lines" metric on the agent-output Timeline item and the details
pane with "N turns", counted off the session's Transcript. The line count comes
from the byte relay counting newline-terminated terminal output, and a
full-screen TUI redraws with cursor movements instead of newlines — which is
why the metric reads 0 for every real session.

A turn is what the Transcript pane draws as one: the agent's prose, its
thinking, a tool call, a tool result, a turn put to it. The backend's own
bookkeeping — about a third of every log — is not counted. The transcript
parser in the render crate already reads lines into turns and counts them
incrementally through its cursor (one log line can be several turns), so the
relay that follows the log keeps a running turn count the same way it keeps
the line count and the latest statement today, and stores it beside them in
the capture summary. The Timeline read stays a read of the stored summary —
no parsing of logs on the way out.

A session that keeps no log — every stub agent, every backend without one —
has no turns to count, and its row shows **no metric at all** rather than a
zero or the old line count. That means the summary distinguishes "no
transcript" from "no turns yet"; the event the web reads carries the count as
absent, not as 0. The stored line count itself stays where it is — only what
is shown changes.

## Acceptance criteria

- [ ] A running claude-backed session's Timeline row and details pane show a
      growing "N turns" (with "1 turn" singular), bookkeeping excluded.
- [ ] A session with no transcript shows no metric at all in either place.
- [ ] The count is kept by the relay as the log is followed and read back off
      the stored summary — timeline reads parse no log lines.
- [ ] Web and server tests cover both the counted and the no-transcript rows.
