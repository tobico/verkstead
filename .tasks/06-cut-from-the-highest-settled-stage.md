# 06. Cut from the highest settled stage

## What to build

A new Stage's branch is cut from **the highest settled Stage in its roadmap's
chain**, rather than from the branch of the Conversation whose settling started
it. It then builds on everything finished so far — its own dependencies among
them — and the rebase at its finish is small.

Today the carry-on that starts a Stage hands the settling Conversation's own
branch to the reading that decides where the branch goes. That reading stays
exactly what it is: whether the repository records a way to stack a Stage, and
whether the default branch already holds the work being built on, are still the
two questions and still decide between standing on a branch, standing on one the
default branch does not hold, and coming off the default branch. **What changes
is what it is handed.**

Four cases it has to get right, and three of them are today's answers unchanged:

- **A roadmap run in order** gives the predecessor, because the predecessor is
  the highest settled Stage in the chain. Nothing about such a roadmap moves.
- **A settled sibling above the predecessor** gives the sibling.
- **Stage 01 of a roadmap whose own pull request is unmerged** comes off the
  roadmap's own branch, as it does today. The chain's bottom *is* that branch
  while it is unmerged — the default branch does not hold the roadmap the Stage
  is started from — and that is why it has to keep coming off it.
- **Nothing unmerged left to stand on** is still the default branch.

One thing parts company here that used to be one fact: **a Stage's base and what
it ends up stacked on are no longer the same thing.** It is cut from the highest
settled Stage, and at its finish it joins on top of whatever the chain's top is
by then. So whatever the record keeps about what a Stage stands on, and whatever
a companion repository's own branch is cut from, and whatever the planning
session is told about where its branch came from, all describe the **base** — and
the words that say so should say *cut from* where they now say *stacks on*, so
that a session reading them is not told something the finish will contradict.

## Acceptance criteria

- [ ] In a roadmap run in order the base is the predecessor, exactly as today,
      and nothing about such a roadmap's start changes.
- [ ] With a settled sibling above the predecessor in the chain, the base is the
      sibling.
- [ ] Stage 01 of a roadmap whose own pull request is unmerged comes off the
      roadmap's own branch, as it does today; nothing unmerged left to stand on is
      still the default branch.
- [ ] A companion repository's own branch for the Stage is cut beside the Stage's,
      off the same Stage's companion branch, as it is today.
- [ ] What the planning session is told about where its branch came from says the
      base it was cut from rather than what it will be stacked on.
