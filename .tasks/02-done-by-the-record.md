# 02. Done by the record

## What to build

What a roadmap has left, as the carry-on asks it when a wrap-up settles, comes
from **the record and the boxes together** rather than from the boxes alone.

**What the record says about one stage**, by the Conversation the label on it
belongs to:

- **Settled** — it is in Done now; or it is Closed and ever reached Wrapping or
  Done. The human's own two halves: nothing may rebase onto a branch that is
  moving again, so a stage steered out of Done is in flight once more; and
  closing a Conversation that got as far as a pull request is the human saying
  they are finished with it.
- **In flight** — anything else that is not Closed: implementing, wrapping,
  answering a question, being followed up.
- **Abandoned** — Closed without ever having reached Wrapping. Neither settled
  nor in flight, and nothing here starts it again.

Where two Conversations answer to one label — a second attempt after one was
closed — settled counts over in flight, and in flight over abandoned. And the
reading is **per Repo**: a Conversation belongs to one, and two Repos may hold
roadmaps of the same name.

**The rule that joins it to the boxes: the record decides where it has a row
for that stage, and the box decides where it has none.**

- A stage **settled and unticked** on the branch being read is done. This is the
  whole point: with branches side by side each worktree holds a different
  `ROADMAP.md`, so the boxes stop being one fact.
- A stage **ticked with no record** of it is done — worked by hand or by the old
  tools, and the boxes are all there is to go on.
- A stage the record says is **in flight** is neither done nor startable,
  whatever its box says. Which is newly load-bearing: the `next-task` skill
  ticks a stage's own box in its **finish** commit, before its pull request even
  opens, so a ticked box already means *its tasks are done* rather than *it
  settled*.

**The annotation stays as the fallback**, which is the human's choice. The
caller's own stage is skipped by its in-flight annotation where the record has
no label for it — every stage started between ADR-0017 landing and task 01 has a
row holding a roadmap and no label, and that skip is the only thing that keeps
one from being offered its own stage back.

**A roadmap run in order behaves exactly as it does today**, which is every
roadmap until stage 04 and is the thing to prove hardest.

The module and function docs that say a stage is ticked off in the plan commit
of the stage after it are wrong already and are corrected here, in the functions
this task rewrites. The words outside this module are task 04's.

## Acceptance criteria

- [ ] A stage whose Conversation is in Done and whose box is unticked on the
      branch being read is done, and the carry-on starts the stage after it.
- [ ] A stage ticked with no record of it is done; a stage the record says is in
      flight is neither done nor startable however its box reads.
- [ ] A stage steered out of Done reads as in flight again; one Closed from
      Wrapping or Done reads as settled; one Closed before it ever wrapped up
      reads as neither.
- [ ] A roadmap run in order carries on exactly as it does today, a stage whose
      row holds a roadmap and no label included — still skipped by its own
      annotation.
- [ ] Two Repos holding roadmaps of one name do not answer for each other.
