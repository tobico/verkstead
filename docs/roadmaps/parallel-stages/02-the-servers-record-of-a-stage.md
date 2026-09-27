# 02. The server's record of a stage

## Goal

Verkstead knows, from its own record, **which stage of its roadmap each stage
Conversation is and whether it has settled**, and decides what is done from
that rather than from the ticked boxes in whichever worktree it happens to be
reading. Each stage ticks only its own box, in its own finish commit. A roadmap
still runs in order, and run in order it behaves as it did — what changes is
where the answer to *is stage 03 done* comes from.

## Decisions in force

- **[ADR-0021](../../adr/0021-parallel-stages.md), *What satisfies a
  dependency***: the server's record decides; the boxes are the score people
  read.
- **This is [ADR-0017](../../adr/0017-a-stage-knows-its-roadmap.md)'s record
  extended**, on the same grounds: a fact Verkstead decided, written once, that
  the repository does not hold in one place. Beside *which roadmap*, *which
  stage*; and that it settled is the Conversation reaching Done.
- **Each stage ticks only its own box, at finish.** The `next-task` skill
  already says so. The `next-stage` skill's *tick every stage above this one
  that is still annotated as in progress* goes, with the justification under it
  — *a stage is only started once the one before it has settled* — which stops
  being true in stage 04.
- **The one-Stage-behind rule is retired**: the **Stage** entry of `CONTEXT.md`
  and the doc on `next_stage` both say the tick rides in the plan commit of the
  stage after, and both are corrected here.
- **The boxes still count where there is no record.** A stage worked before
  this, by hand or by the old tools, has a ticked box and no row; a ticked box
  is a stage done. The record adds to what the boxes say, and is what is
  believed where a box is unticked and the record says settled.
- **Settled, not merged**: a stage whose pull request is open and whose wrap-up
  is Done is done.

## Proposed tasks (provisional)

1. **Which stage, written down** — a stage Conversation's label is recorded
   when it starts, at the three moments ADR-0017 names. AC: a carried-on stage,
   an adopted one and stage 01 of a roadmap just written each have it; a stage
   from before this has none and nothing breaks.
2. **Done by the record** — what a roadmap has left is read from the record
   and the boxes together. AC: a stage settled and unticked on the branch being
   read is done; a stage ticked with no record is done; a stage in flight is
   neither done nor startable.
3. **A stage ticks itself and nothing else** — the `next-stage` skill stops
   ticking the stages above it. AC: the skill's assertions in `skills.rs`
   cover the new wording; a plan commit changes only its own stage's line.
4. **The words corrected** — `CONTEXT.md`'s **Stage** and the docs in
   `stages.rs`. AC: neither says the score is kept one stage behind.

## Re-verify at start

- `stage_roadmaps` is still keyed on the Conversation and the store still has
  no migration machinery for a `STRICT` table, so a new fact is still a table
  of its own — unless stage 07 of the processes roadmap, which rekeyed tables
  through `migrations.rs`, has changed what the convention is.
- `next_stage` still skips the caller's own stage by its annotation. With the
  record in hand that skip may have nothing left to do.
- Whether the in-flight annotation is still worth writing, the server no
  longer needing it: it is still what tells a person reading the file which
  stage is being worked. Put it to the human in the stage's own breakdown Set
  rather than deciding it here.
- `startable` and the adoption path read the boxes too, and have to agree with
  the carry-on about what is done.
