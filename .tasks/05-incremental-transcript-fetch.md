# 05. Incremental transcript fetch

## What to build

Stop a running session's open Transcript re-downloading its whole record on
every batch — megabytes late in a session, twice a second, over the network
the tool is used across.

The transcript endpoint grows an incremental form: the client passes a cursor
saying where its reading got to, and the server returns only what lies past
it, plus the new cursor. The cursor is the server's and opaque to the client
(the natural unit is how far into the session log rendering has read; turns
are already keyed by their place in the conversation). First open reads whole;
every Nudge-driven re-read while the session runs fetches only the new turns
and appends them through the reconcile merge already in place. Bookkeeping
lines keep folding into their single end group even when they arrive across
increments. A finished session stays frozen (`static`) exactly as today, and
the race the pane already guards against — the last words arriving with the
session's end — must stay closed: a full re-read is always a correct fallback.

The Capture stays whole-refetch: it stands in only for sessions that kept no
log, and incremental reads stop at the Transcript until that pane hurts.

## Acceptance criteria

- [ ] With a running session's Transcript open, re-reads request only turns
      past the cursor and responses shrink accordingly (first open still
      reads whole)
- [ ] Appended turns merge into the drawn record — open folds stay open, and
      bookkeeping still gathers into one end group across increments
- [ ] A finished session's record is byte-identical whether read whole or
      accumulated incrementally, ending included; golden fixtures cover the
      incremental wire shape
- [ ] Cursor mismatch or any incremental failure degrades to a whole read,
      never a gap in the record
