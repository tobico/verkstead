# 02. Steer

## Goal

The Steer control, end to end: a row in the Conversation's menu beside Stop
that stops the drive and opens a modal; submitting moves the Conversation into
the chosen state, recreating what is missing, clearing any stop, landing a
Steer Event, and resuming in the same press. Demonstrable on the four
exceptions that motivated it — mark a missed merge Done, steer follow-up work
into Implementing with a hand-written instruction, steer into Wrapping for a
fresh review, steer into Grilling with a new brief.

## Decisions in force

All from [ADR-0010](../../adr/0010-one-stop-and-steer.md); what bears on this
stage:

- **Targets and sources.** Targets are Grilling, Implementing, Wrapping and
  Done — Draft and Closed are not targets, each having its own way in.
  Sources are *any* state, recreating what is missing: a Worktree checked out
  from the branch where the directory has gone, the branch itself for a
  Draft. Steering a closed Conversation is (after stage 03) the one way back
  in.
- **Stop at click.** Clicking Steer stops the drive before the modal opens —
  nothing new launches; a running session is seen out unless the modal's
  **Interrupt current task** checkbox force-stops it. Cancel leaves the
  Conversation stopped, Resume on offer; that is accepted, not a bug — the
  click freezes the world while the human composes.
- **Into Grilling**: optional new brief, landing as a new round's Brief Event
  frozen at once; the digest of everything already answered is offered as a
  choice, not always sent.
- **Into Implementing**: where a backlog or roadmap stands, the human may
  choose to continue it; otherwise a hand-written instruction is required.
  The instruction session is a *pipeline driver*, unlike the old Manual
  Task: registered as driving, judged by the ordinary end-of-session rules,
  and on a clean finish the pipeline carries on from whatever the branch
  then holds — wrap-up where a pull request exists, the next task where the
  backlog holds one.
- **Into Wrapping**: no payload; the wrap-up's watchers recompute, as a
  pressed Resume does today.
- **Into Done**: the move alone — nothing resumes.
- **The Pairing** for the role steered into is shown, prefilled from the
  Conversation's own, and what is picked is **recorded as the
  Conversation's** — steering re-settles what runs the work. A steered Draft
  has none fixed yet, which is why the pick is part of the modal rather than
  an error path.
- **The record.** The steer lands as its own Timeline Event — the human moved
  it — carrying the brief or instruction as its body, distinct from the
  machine's plain Moved line.

## Proposed tasks (provisional)

1. **Store: the Steer Event and the move.** Event kind carrying target and
   payload; the state move; a new-round Brief on a Grilling steer; recording
   the picked Pairing as the Conversation's.
   - A steer to Grilling with a brief leaves a new frozen Brief Event plus a
     Steer Event; one without leaves the Steer Event alone.
2. **Server: the two presses.** The click endpoint (stop the drive, report
   what is running) and the submit endpoint (force-stop if asked, recreate
   branch/worktree, clear the stop, move, record, resume).
   - Steering a Conversation whose Worktree directory is gone recreates it
     from the branch and proceeds.
3. **Launch paths per target.** Grilling primed with the new brief and the
   chosen digest; Implementing as continue-backlog or instruction session;
   Wrapping afresh; Done nothing.
   - An instruction session that commits and goes quiet hands the pipeline
     on, and the branch's pull request is wrapped up again.
4. **Workbench: the modal.** Menu row, target picker, payload fields shown
   per target, Pairing pick, Interrupt checkbox; cancel leaves the stopped
   Conversation with Resume drawn.
5. **The vocabulary this stage adds.** CONTEXT.md gains a **Steer** entry —
   what it moves, what it recreates, what it records — and **Resume**'s is
   amended: it sends the reader to Manual Task for steering the work, which is
   Steer's job from here and stage 03's to retire.
   - Steer's entry is written against the same states the modal offers.

## Re-verify at start

- Stage 01 landed: there is one stop to clear, not two.
- The shape of `resume.rs`'s per-state recomputation and `grillings::again`'s
  digest priming — the steer launch paths reuse them rather than fork them.
- What `runner.rs` launches with (`Prompt::*` variants) and how a session is
  registered as driving.
- Whether Reopen still exists (it does until stage 03) — Steer must not
  collide with it on a Done Conversation in the meantime.
