# 04. A stage ticks itself, and the words

## What to build

The `next-stage` skill **stops ticking the stages above the one it is planning**.
The bullet goes, and so does the justification written under it — *a stage is
only started once the one before it has settled* — which stops being true in
stage 04. Each stage ticks only its own box, in its own finish commit, which is
what the `next-task` skill already does and is left alone.

**The annotation stays**, which is the human's choice: a stage is still
annotated in progress with the branch it is worked on, beside whatever that line
already declares rather than over it. Verkstead no longer needs it to know which
stage is being worked, and two things still do — a person reading `ROADMAP.md`,
and the readings themselves wherever there is no record to go on.

The assertions in `skills.rs` follow the wording, the way they follow every
skill's wording: the one that has the stage before this one ticked goes, and
what takes its place is that a plan commit moves only its own stage's line.

**And the one-stage-behind rule is retired from the words that state it.**
`CONTEXT.md`'s **Stage** entry says a stage is done when its box is ticked,
*which is the roadmap's own score and is kept one Stage behind: the tick rides
in the plan commit of the Stage after it*. Neither half survives — the tick
rides in the stage's own finish commit, and what says a stage is done to
Verkstead is the record, with the boxes behind it where there is no record. The
`next-task` skill's *Verkstead reads those boxes to decide what to start next*
wants the same correction, in a sentence rather than a section.

Nothing about the viewer is this task's, or this stage's: the card and the
details pane still draw the boxes, and showing every stage in flight off the
record is stage 06.

## Acceptance criteria

- [ ] The `next-stage` skill ticks no stage but its own and still annotates this
      one in progress, declaration intact.
- [ ] `skills.rs` covers the new wording, and nothing in it asserts that the
      stage before this one is ticked.
- [ ] `CONTEXT.md`'s **Stage** entry no longer says the score is kept one stage
      behind, and says what decides that a stage is done.
- [ ] The `next-task` skill still ticks its own stage's box at finish, and says
      nothing that reads as Verkstead deciding from the boxes alone.
