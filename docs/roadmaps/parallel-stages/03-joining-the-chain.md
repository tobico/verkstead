# 03. Joining the chain

## Goal

A stage whose tasks are all done **joins its roadmap's chain before its pull
request opens**: it waits until every stage already in the chain has settled,
rebases onto the top, and only then is pushed, opened and wrapped up. A new
stage is cut from the highest settled stage. In a roadmap run in order — which
is every roadmap until stage 04 — the top of the chain is what the stage was
cut from, the rebase moves nothing, and nobody sees a difference. Demonstrable
by starting two stages of one roadmap by hand and finishing them in either
order.

## Decisions in force

- **[ADR-0021](../../adr/0021-parallel-stages.md), *The chain***: one chain
  per roadmap, in the order its stages finish.
- **It never rewrites a pull request somebody has started reading.** That is
  the whole reason the join is at the finish rather than at the start of the
  stage that needs both, which is where the Brief first had it. A stage is
  rebased once, before it has a pull request. *Without compromising the review
  process* is the human's condition on the whole effort.
- **The wait is on settling**, the human's own addition: a stage below that is
  still wrapping up — a finding being fixed, a check going green — is a branch
  still moving, and nothing rebases onto it. So the server holds the finish
  step; it is not the session's to judge.
- **One at a time, in the order the tasks finished.** A second stage waiting to
  join waits on the first joining *and settling*.
  **The joins are the one thing the roadmap does in single file**, so a stage
  whose wrap-up cannot finish holds up every later stage's join, dependent on it
  or not. Only the join: a waiting stage's own work, its checks and its review
  wait on nobody. That is the price of never rebasing a branch anybody is
  reading, and the Timeline says which stage is being waited on so it does not
  read as a stall.
- **A joined stage is never rebased by a later one.** What moves it afterwards
  is what moves a stacked branch today: `gh stack sync` after the default
  branch moves or a pull request below merges.
- **Cut from the highest settled stage in the chain**, not from the highest of
  the stage's own dependencies: it builds on everything finished, and the
  rebase at the finish is small. **The chain starts at the roadmap's own
  branch** — the Conversation that wrote the roadmap, while its pull request is
  unmerged — so stage 01 comes off there, exactly as it comes off its
  predecessor today and for the same reason: the default branch does not hold
  the roadmap the stage is started from. The default branch only where nothing
  unmerged is left to stand on.
- **A conflict at the join is the joining session's to resolve**: resolve it,
  run the checks again, ask the human only where it cannot tell which side is
  right.
- **`gh stack` is the session's to run, never the server's**, and how a branch
  joins a stack is the target repository's `docs/agents/git-workflow.md`. A
  repository recording no stacking mechanism gets the rebase and a pull request
  based on the branch below, as the unrecorded case is handled today.
- **A waiting stage has tests to run and a branch to keep**: companions'
  branches join where the stage's own does.

## Proposed tasks (provisional)

1. **Waiting to join** — a stage with every task ticked and a stage below
   unsettled is held before its finish step, and says so. AC: the finish
   session is not dispatched; the Conversation reads as waiting to join and on
   which stage; it is released when that stage settles.
2. **One at a time** — two stages waiting are released in the order their
   tasks finished. AC: the second is released only once the first has settled;
   a restart of the server keeps the order.
3. **The join** — the finish step rebases onto the top of the chain before
   anything is pushed, by the repository's own stacking block where it has
   one. AC: the pull request's base is the stage below; a stage cut from the
   top rebases to nothing; the branch is never force-pushed after its pull
   request opens.
4. **A conflict on the way in** — the finish skill says what to do with one.
   AC: a conflict the session resolves ends in green checks and a pull
   request; one it cannot judge ends in a Question Set.
5. **Cut from the highest settled stage** — the carry-on's choice of base.
   AC: in a roadmap run in order the base is the predecessor, as today; with a
   settled sibling above the predecessor, the base is the sibling; stage 01 of a
   roadmap whose own pull request is unmerged comes off the roadmap's branch, as
   it does today.
6. **This repository's own block** — `docs/agents/git-workflow.md` says how a
   stage joins. AC: the block covers a branch that exists and is not at the
   top.
7. **The words** — `docs/design/verkstead.md`'s **Stages always stack**, whose
   *the next stage's branch stacks on the unmerged predecessor* is what the chain
   replaces. AC: it is refined in the document's own *refined <date>, building
   <stage>* form rather than rewritten, and says the join is at the finish.

## Re-verify at start

- Stage 02 landed: *settled* is read from the record.
- `Stands` and `standing` in `continuing.rs` are still where a stage's base is
  chosen, and `stage_branches.stacks_on` is still where it is recorded. A
  stage's base and what it ends up stacked on stop being the same fact here.
  `standing` is handed the **settling Conversation's** branch as the
  predecessor, which is how stage 01 comes off the roadmap's own branch today
  and how it has to keep coming off it — the chain's bottom is that branch, not
  the default one.
- `Step::Finish` and `to_a_pull_request` in `runner.rs` are still the finish,
  and the hold belongs in front of them.
- `gh stack` keeps its registry per worktree, so a session re-adopts the chain
  with `gh stack init` before it can add to it. Whether `gh stack` can adopt a
  branch that was cut from the middle of the chain, once rebased, is to be
  proven before the skill is written around it.
- A stage waiting to join is a session not running and a Conversation not
  stopped. Check what the stall detection in `drivers.rs` makes of it.
- The **Fix Merge Issues** walk of a stack still reads the chain through `gh`
  and records every link.
