# 01. Waiting to join

## What to build

The **chain** on the server's record, and the hold that stands in front of a
Stage's finish while a Stage already in the chain has not settled.

A roadmap's chain is one chain of branches in the order its Stages finish, its
bottom the roadmap's own branch while that is unmerged — see
[ADR-0021](../docs/adr/0021-parallel-stages.md), *The chain*. A Stage **joins**
it when its finish pushes and opens its pull request. Stage 02 put *which
Stages settled* on the record; nothing yet records *which have joined*, or in
what order, and both this hold and the base a later Stage is cut from read that.
So this task puts it there: a Stage is recorded as joined where its pull request
is first recorded against it, which is what already happens when a finish opens
one, and the chain reads back bottom to top out of those rows plus the roadmap's
own branch.

Then the hold. When a Stage's every box is ticked, the run's next step is the
finish — and before a session is launched for it the chain is read. Where any
Stage already in it has not settled, **no session is launched**: the Stage sits
there, its Timeline says it is waiting to join and names the Stage it is waiting
on, and the Conversation reads as *waiting to join* rather than as standing
still. When that Stage settles, the held Stage is released and its finish runs
exactly as it does today.

The hold is the server's judgement and not the session's — a Stage below that is
still wrapping up is a branch still moving, and nothing rebases onto one. Three
things about where it goes:

- **A finish session is launched from two places**, and the hold stands in front
  of both: the run's own loop over the backlog, and the take-up a Resume makes
  on a Conversation whose boxes are all ticked. The step that carries the work
  to its pull request runs *after* the finish session, so it is not where the
  hold belongs.
- **Only a Stage is held.** An ordinary feature's backlog has no roadmap and no
  chain, and its finish is untouched.
- **Nothing reads a held Stage as stalled.** The run holds its driver
  registration across the whole loop, so a Stage waiting inside it still counts
  as driven, and a restart takes it up through the ordinary resume and finds it
  held again. Check that rather than assume it.

*Waiting to join* is a condition of Implementing rather than a state, drawn the
way *Waiting on checks* is drawn for a wrap-up: a line on the Timeline, a label
on the Conversation, nothing on the Lifecycle and no push to the devices. What
every Stage of a roadmap is showing is stage 06's and not this task's.

In a roadmap run in order the Stage below has always settled before the next one
finishes, so nothing is ever held and nothing about such a roadmap changes.

## Acceptance criteria

- [ ] A Stage with every box ticked and a Stage already in its roadmap's chain
      that has not settled gets no finish session; its Timeline says it is
      waiting to join and names the Stage or Stages it is waiting on.
- [ ] That Stage settling releases it, and the finish step then runs exactly as
      it does today — same session, same skill, same pull request.
- [ ] The hold stands in front of both launch sites: the run's loop and a Resume
      pressed on a Stage whose boxes are all ticked.
- [ ] A held Stage is not read as stalled and is not stopped; a restart of the
      server takes it up and finds it still held.
- [ ] A roadmap run in order holds nothing, and an ordinary feature's backlog is
      untouched.
