# 04. Steer into Investigating

## What to build

**Investigating on the Steer form, from every state, and the state it was steered
from written down beside the Steer Event.** A question about work can arise at
any point in it, so this target is offered wherever the form is drawn — unlike
wrapping up and following up, neither of which is offered without a pull request.
Reading the source state back off the Timeline is the thing this task exists to
avoid: the ending is the one moment nothing else is there to say where the work
came from, and the steer is the moment the fact is known.

**The target.** A row on the form's target list with a note saying what an
investigation is, and a row in the enum the server refuses by — work goes on in
it, so it needs a Pairing settled and a Worktree to run in, and a Worktree is
made where none stands exactly as a steer into any working state makes one. The
role it settles is Implementation. The state-to-target mapping both ways gains
its row.

**The brief is required**, as a follow-up's is and for the same reason: there is
nothing on the branch that could stand for it, an investigation being a thing the
human wanted rather than a step of the run. A submit that names Investigating
without one is refused by name. Whitespace alone is nothing written.

It lands as the Steer Event's own body, which is where every written payload
lands, and the session started on it is primed with that brief as its Brief.

**The half-written form keeps it in a slot of its own.** The form saves itself as
it is typed and the record holds a slot per target's payload, so switching target
loses nothing — Investigating gets a column of its own, with the one-time rewrite
beside it that gives every row already in the field the value that says what was
true of them before the column existed.

**The source state goes in a table of its own, keyed by the Steer Event**, the way
the steer's companion rows are — the steer's own record has no room for a column
and no machinery to add one. Written in the same transaction as the move, because
a steer that recorded where the work went without recording where it came from
would be half an account of one press. Read back with the rest of the steer's
record, and a Steer Event with no row is a steer made before any of this was
written down rather than an error.

**And the session is launched**, under the Implementation Pairing on the
`investigating` skill, spawned as a steer into Follow-up spawns its own with the
registration handed to the run that drives the rounds.

The ending still lands Done after this task, which is right for a steer from a
Draft or a closed Conversation and wrong for every other source. Task 05 is what
reads the record this one writes.

## Acceptance criteria

- [ ] The Steer form offers Investigating on a Conversation in every state it can
      be opened on, including ones on no pull request, with the Implementation
      picker drawn under it.
- [ ] A submit naming Investigating with no brief is refused by name, and the
      Conversation does not move.
- [ ] A Done Develop Conversation steered into Investigating reads Investigating,
      has a Worktree, and has one session running whose prompt carries the brief
      the human typed; the Steer Event's body is that brief.
- [ ] The state the Conversation was in at the submit is on the record beside that
      Event, reads back with the rest of the steer's record, and survives a
      restart.
