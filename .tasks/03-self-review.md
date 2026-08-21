# 03. The wrap-up self-review

## What to build

The review that makes wrap-up a phase rather than a wait. There are no
per-commit review states anywhere in Verkstead — commits are events to read, and
this is where problems get raised instead.

When a Conversation enters Wrapping, one session starts inside a new bundled
**reviewing** skill, under the implementation Profile. Fresh context is the
whole point: the sessions that wrote the work each saw one task, and none of
them saw the branch. This one reads the PR — its diff, the code around it, what
the repository says about itself — and looks for what a reviewer would.

**It reviews and does not fix.** Nothing it finds is changed by this session.
What it produces is one Question Set on the Timeline, through `verkstead ask`,
with a Question per finding and Options that amount to *fix this* or *leave it*.
That is what puts the human in the loop without putting them at a terminal:
they answer from the phone like every other Set.

The answers are what turn findings into work. Each finding the human accepts
dispatches an addressing session — the same bundled skill task 02 wrote, given
the finding as its feedback. Findings they decline dispatch nothing and are not
raised again.

A review that finds nothing worth raising asks nothing: it says so on the
Timeline and wrap-up carries on. A Set with no Questions in it would be a row
for the human to dismiss, and the point of the phase is to spend their attention
only where there is a decision.

## Acceptance criteria

- [ ] Entering Wrapping starts exactly one review session, in a fresh context,
      under the implementation Profile inside the bundled reviewing skill.
- [ ] The reviewing skill exists and tells the session to review rather than
      change: nothing it finds is committed by it.
- [ ] Its findings arrive as one Question Set on the Conversation's Timeline,
      one Question per finding, answerable in the workbench and on the phone.
- [ ] An accepted finding dispatches an addressing session carrying that
      finding; a declined one dispatches nothing and is not re-raised.
- [ ] A review that finds nothing raises no Question Set and says so on the
      Timeline.
- [ ] The review's Set being answered is recorded as one of the things wrap-up
      waits on.
