# 03. The running session's pairing on the record

## What to build

When the server starts a session it has the pairing in hand — the profile and
the model the session runs under. Stamp the profile's name and the model id
onto the session's agent-output event as it is written, so they are part of the
record and survive a server restart, and carry both on `AgentOutputEvent` over
the wire.

Rows written before this change carry neither; they come down null (or absent)
and every reader of an agent-output event copes. This was settled over the
alternative of a live registry field precisely because the record outlives the
process — what ran, and under what, is history worth keeping, not just status
worth showing.

The model travels as the raw id (`claude-opus-5`); prettifying is the viewer's,
via task 01's helper. Nothing in the viewer consumes the new fields yet — task
04 does — so this slice is demonstrated by a fresh session's event carrying
both fields and old records still reading cleanly.

## Acceptance criteria

- [ ] A newly started session's agent-output event carries the profile name and model id, on the record and on the wire
- [ ] Events from before the change come down without them and every existing view still renders
- [ ] Regenerated TypeScript types; server tests cover the stamped fields and the null case
