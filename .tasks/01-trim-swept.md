# 01. Trim, swept

## What to build

The trim itself, end to end: a store operation that takes the bulk out of one
archived Conversation, and a periodic cleanup sweep in the server that applies
it to everything archived longer than a built-in three days.

**What trim removes, and what it keeps.** The rule the grilling settled: *what
a Timeline card renders survives; what only a drill-down shows goes.* Removed,
per event of the Conversation: the full agent-output chunks and turns, the
verbatim transcript lines, and the session-name records. Kept: the timeline
events themselves, the capture summary an agent-output card is drawn from, the
records of which account and model ran, every Question Set with its Response,
the commit records and summaries, and the pull-request rows. Verify the
boundary against what the cards actually read rather than trusting this list —
the card is the rule, the list is the grilling's expectation. A trimmed
Conversation must still Share exactly as before, a Share never having included
what trimming takes.

**The mark.** A sidecar table beside the archivings (the store's stance: a new
per-conversation fact is a sidecar, never a column), holding the conversation
and when it was trimmed. Give the operation an outcome enum in the archives
module's style — trimmed, already trimmed, not archived, no such conversation —
and refuse to trim what is not archived.

**Trimmable again.** A fresh archiving restarts everything: the sweep trims
when `archived_at` is older than the duration *and* the trim mark is either
absent or older than `archived_at`, so a Conversation steered back to life and
put away again has its new bulk taken too. Unarchiving removes the archive row
and thereby stops the clock; the trim mark stays, because the data is gone.

**The sweep.** A new server module on the merge-sweep pattern: spawned from
the router's startup alongside the other sweeps, guarded by the runs-sessions
check so test routers run no loops, paced by its own field on `Pace` so tests
can drive it fast. An hourly pass is plenty for day-granularity clocks. The
three-day threshold is a constant in this task; task 03 moves it behind the
settings. The existing backlog is cleaned deliberately: the very first pass
trims everything already past the threshold. The sweep says what it trimmed in
the log and nowhere else — it refuses for nothing and returns nothing.

## Acceptance criteria

- [ ] A store test trims an archived Conversation and shows the chunk,
      transcript and session-name rows gone while every card-feeding row —
      capture summaries, sets, responses, commits, pull requests — survives;
      trimming again reports already trimmed; trimming an unarchived
      Conversation is refused.
- [ ] A sweep test at test pace trims an old archive, skips a fresh one, an
      unarchived one and an already-trimmed one, and trims again a
      Conversation re-archived after its last trim.
- [ ] Sharing a trimmed Conversation produces the same Share as before the
      trim; the sweep logs each trim and writes nothing to any Timeline.
