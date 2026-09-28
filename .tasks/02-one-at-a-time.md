# 02. One at a time

## What to build

Two Stages waiting to join are released **one at a time, in the order their
tasks finished** — and a second waiting Stage waits on the first *joining and
settling*, not merely joining.

Task 01 holds a Stage while a Stage already in the chain is unsettled. That is
half the rule: with two Stages finishing close together, both are held and both
would be released by the same settle. So the moment a Stage's tasks finished is
written down as it enters the hold, and that order is what lets them in. The
first is released when the chain is settled; the second stays held until the
first has joined **and** its own wrap-up has settled, which is the same
condition task 01 already reads — a branch still moving is not one to rebase
onto, and a Stage that has just joined is moving until it settles.

**The order is a stored fact rather than a queue in memory**, because the wait
may be long and a server restart must not reorder it. A restart takes each held
Stage up again through the ordinary resume, and what decides which of them goes
first is the record of when each one's tasks finished rather than the order the
resume happened to reach them in.

What this costs is worth saying on the Timeline plainly, because it is the price
the whole design pays: a Stage whose wrap-up cannot finish — an unanswered
review Set, a check that stays red — holds up every later Stage's join, whether
or not anything depends on it. **Only the join is held.** A waiting Stage's own
work, its checks, its comments and its review wait on nobody; it is the joining
and nothing else that is in single file.

## Acceptance criteria

- [ ] Two Stages held at once are released one at a time, the one whose tasks
      finished first going first.
- [ ] The first Stage joining does not release the second; the first Stage
      settling does.
- [ ] A restart of the server releases held Stages in the same order it would
      have without the restart.
- [ ] Each waiting Stage's Timeline names what it is waiting on, so a queue of
      two reads as a queue rather than as two stalls.
