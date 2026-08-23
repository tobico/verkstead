# 07. Reopening rounds

## What to build

A Done Conversation can be reopened with a new brief round.

Done means Verkstead has finished with the work, not that the work is finished
for good: something comes back, or the next piece of it belongs to the same
branch. Reopening starts a second round on the same Conversation — a new Brief,
grilled as the first was, run as the first was.

The new Brief is a **new Event**, never an edit of the old one. The first Brief
froze when its grilling started and it stays frozen: it is what that round was
built from, and a Timeline that lost it would lose why the work is the shape it
is. So the Conversation gains a second Brief, editable until the new round's
grilling starts, and freezing then in its turn.

Reopening is offered on Done and nowhere else. Aborted is off the ladder and
stays there; the other states are somewhere the work has got to, and there is
nothing to reopen.

The Worktree is ordinarily still there — it is removed only when a Conversation
is aborted — so recreating one is the fallback rather than the path: a Worktree
whose directory has gone, by hand or by a machine rebuilt, is made again on the
Conversation's existing branch rather than on a new one. A branch that has been
worked is not a branch to start over.

The Timeline shows where one round ends and the next begins, so a reader can
tell which Brief the work under it was built from.

## Acceptance criteria

- [ ] Reopening is offered on a Done Conversation and on no other state, and it
      leaves the frozen Brief untouched while adding a new one that is editable
      until its round's grilling starts.
- [ ] A reopened Conversation whose Worktree directory has gone gets one back on
      its existing branch, and one that still has its Worktree keeps it.
- [ ] The Timeline says where the round boundary falls, and the second round
      runs the ordinary pipeline from grilling onward.
