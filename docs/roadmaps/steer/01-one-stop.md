# 01. One stop

## Goal

A Conversation is driven or it is stopped, and the workbench says so with one
concept: Halt and Pause merged into a single stop, one badge, one Notice, one
Resume — and the Hold gone, with nothing in its place. Demonstrable end to
end: exhaust a usage window and press Stop on another Conversation, and both
cards read as the same kind of stopped thing; press Stop and type into a live
Screen, and nothing ends the session or advances the run under you.

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
- **A usage-window stop ends its session.** `limits::pause` has never touched
  the session it stopped — the agent's own wait was not Verkstead's to end —
  and what kept the two in step was the sweep relaunching at the same reset,
  `Sessions::start` ending whatever was registered. With the sweep gone the
  session has to be ended where the stop is written, or the agent wakes at the
  reset and works on inside a Conversation that reads as stopped.
- **Notification rules unchanged.** A stop Verkstead decided on is pushed;
  the human's own press and a stop nobody chose are not.
- ***Blocked on you* badges any stopped Conversation.**
- **The Hold is retired** (this amends ADR-0007), and nothing replaces it.
  Typing into a Screen commits Verkstead to nothing — no register, no
  hand-back, no badge, and no clock a keystroke puts back. Somebody who wants
  to intervene by hand presses **Stop** first, and the one stop is what holds
  the run off while they do; a session typed into while the run is still
  driving it is ended and advanced by the ordinary rules.
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
   - An exhausted window leaves a stop with the Profile and reset words on it,
     and leaves no session running behind it.
   - Resume clears any stop, whatever wrote it.
3. **Workbench: one stopped presentation.** Fold `Pause.tsx` into the one
   stopped card; one badge; reset time drawn as text beside the ordinary
   Resume.
   - A paused-by-window and a pressed-Stop Conversation draw the same shape.
4. **Retire the Hold.** Delete `hold.rs` and the hand-back UI; drivers stop
   waiting on `until_handed_back`. Nothing takes its place, so the Screen's
   input path keeps writing what it is sent straight through — and telling a
   keystroke from a mouse report there was the Hold's business alone, so what
   is left of that distinction goes with it. Its push goes too:
   `News::Waiting`, `push::when_it_has_stood`, `push::HELD_A_WHILE` and
   `Pace::holding` have nothing left to announce, and `screen.rs` is the one
   caller.
   - Typing into a driven session's Screen changes nothing about when it ends;
     pressing Stop first is what holds the run off.
   - Nothing to hand back anywhere, and no device is told about a Hold.
5. **The vocabulary this stage retires.** CONTEXT.md: fold **Halt** and
   **Pause** into one entry for the one stop, delete **Hold**, and follow the
   cross-references out — **Stalled** is defined against both of the first
   two, **Notice** and **Resume** name the Halt, and **Timeline** and
   **Screen** carry the Hold's carve-outs. Naming the merged entry is part of
   the task: *Stop* is already the press, so the state and the button need
   telling apart.
   - No entry describes a Hold, a self-resuming Pause, or two kinds of stop.
   - No *Avoid* line sends a reader to a term this stage retired.

## Re-verify at start

- The `halts.rs` / `pauses.rs` store APIs and the `limits.rs` recognition
  tests as they stand then — the exhausted-window wording is the backend's
  and moves.
- Where `until_handed_back` is awaited (each driver gates itself; sweep for
  all of them, not the remembered list).
- Whether any new stop writer landed since 2026-08-25 (wrap-up watchers,
  checks) that must write the merged stop.
- Push-notification wording that names "paused", "halted" or "held".
