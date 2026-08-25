# 01. One stop

## Goal

A Conversation is driven or it is stopped, and the workbench says so with one
concept: Halt and Pause merged into a single stop, one badge, one Notice, one
Resume — and the Hold gone, replaced by the keystroke-resets-the-quiet-clock
rule. Demonstrable end to end: exhaust a usage window and press Stop on
another Conversation, and both cards read as the same kind of stopped thing;
type into a live Screen during a quiet spell and the session is not ended
under you.

## Decisions in force

All from [ADR-0010](../../adr/0010-one-stop-and-steer.md); what bears on this
stage:

- **One stop concept.** A stop keeps its Notice and remembers whether anybody
  decided it. The restart rule is unchanged in substance: a restarting server
  takes up only the stops nobody chose — a crash, a driver a restart took
  away — and leaves deliberate ones waiting.
- **No stop resumes itself.** The usage-window detection (`limits.rs`) stays,
  but it now writes an ordinary stop naming the Profile and carrying the
  reset time *as words to show*; the five-hour wait that resumed the run when
  the reset passed is deleted. Every stop ends by a press.
- **Notification rules unchanged.** A stop Verkstead decided on is pushed;
  the human's own press and a stop nobody chose are not.
- ***Blocked on you* badges any stopped Conversation.**
- **The Hold is retired** (this amends ADR-0007). Its replacement is a rule:
  a keystroke into a Screen resets the session's quiet clock. No register, no
  hand-back, no badge — and, accepted explicitly in the grilling, no
  protection once the session has exited: the end-of-session judgment then
  runs on whatever was left.
- **Old records stay readable** (ADR-0006's rule): existing Pause Events on
  Timelines are the record of what happened and are not rewritten; the stored
  halt and pause rows are read into the one concept rather than destroyed.

## Proposed tasks (provisional)

1. **Store: the one stop.** Merge `halts.rs` and `pauses.rs` into a single
   stops model — Notice text, the decided-by bit, optional reset-time text.
   - At most one open stop per Conversation, however it arrived.
   - The restart query returns only stops nobody decided.
   - Existing halt and pause rows read back as stops; no row is rewritten.
2. **Server: writers and clearers.** `limits.rs` writes a stop and its wait
   task goes; `stops.rs`, `resume.rs` and the stalled sweep write and clear
   the one thing.
   - An exhausted window leaves a stop with the Profile and reset words on it.
   - Resume clears any stop, whatever wrote it.
3. **Workbench: one stopped presentation.** Fold `Pause.tsx` into the one
   stopped card; one badge; reset time drawn as text beside the ordinary
   Resume.
   - A paused-by-window and a pressed-Stop Conversation draw the same shape.
4. **Retire the Hold.** Delete `hold.rs` and the hand-back UI; the Screen's
   input path touches the session's `Quiet` clock; drivers stop waiting on
   `until_handed_back`.
   - Typing during a quiet spell keeps the session alive; nothing to hand
     back anywhere.

## Re-verify at start

- The `halts.rs` / `pauses.rs` store APIs and the `limits.rs` recognition
  tests as they stand then — the exhausted-window wording is the backend's
  and moves.
- Where `until_handed_back` is awaited (each driver gates itself; sweep for
  all of them, not the remembered list).
- Whether any new stop writer landed since 2026-08-25 (wrap-up watchers,
  checks) that must write the merged stop.
- Push-notification wording that names "paused" or "halted".
