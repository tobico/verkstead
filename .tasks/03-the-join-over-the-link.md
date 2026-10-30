# 03. The join over the link

## What to build

A stage finishing on a device other than its home asks home, over the link,
whether it may join and what it joins on top of. The finish and the wrap-up run
on the far device (ADR-0029, *The work stays where it is*), so the join has to
be askable from there.

**Today's join is all local reads.** The hold in front of a finish (`joins`)
takes a place in `stage_joinings`, then waits while `ahead_of` — the chain
(`stage_chain`, read off `pull_requests` rows ordered by their Event id), the
queue (`join_queue`) and the standings — names a stage in front; then
`joining` hands the finish session the chain's branches, bottom to top, with the
foot (`stacks_on`) under them. On a copy away from home every one of those reads
is of the wrong database.

**What changes for a copy away from home:**

- The hold asks home instead. Home queues the **tombstone** (the queue is home's
  — the place the slice carried across is ignored) and answers what is ahead
  and, once nothing is, the chain's links and foot. Which branches have merged
  stays git's reading, made on the far device as today.
- **Home unreachable is a wait, said** on the Timeline, under the same
  *Waiting to join* label, and the hold goes on when home answers. Never a
  refusal. Read `docs/copy.md` before writing the sentence.
- **The pull request the join opens is reported to home** over task 01's route,
  with the stage's branch as the record names it, so home's chain gains the far
  stage and the next stage at home stacks on it. The order home's chain reads it
  in is the order home learned of the join.

A stage at home joining is unchanged.

## Acceptance criteria

- [ ] A stage that joins on the far device opens its pull request against the branch home named.
- [ ] With home asleep, the far finish is held and the Timeline says it is waiting on home; it goes on when home answers.
- [ ] The next stage to join at home joins on top of the far stage's branch.
- [ ] Home's queue orders the far stage by when it asked to join, against stages finishing at home.
