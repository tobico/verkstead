# 04. The join

## What to build

The finish step **rebases onto the top of the chain before anything is pushed**,
and only then pushes, opens its pull request and wraps up.

Task 01 released the Stage once the chain had settled; this is what it does when
it is let in. The server knows what the chain is — it recorded it — and the
session is the one that runs `gh stack`, so the chain is told to the session and
the rebase is the session's to do. Which means the finish step's prompt carries
the chain, bottom to top, and says plainly that the rebase comes before the push.
The step that a session runs for a task and the step it runs for the finish are
told apart by the runner already, so there is a place for the finish of a Stage
to be told something a task is not.

What the session does with it comes from the target repository's own
`docs/agents/git-workflow.md` — the block task 03 wrote. `gh stack` is the
session's to run and never the server's, and Verkstead invents no stacking
mechanism of its own.

**Where the repository records no stacking mechanism the rebase still happens**,
it being Verkstead's rather than the block's, and the pull request opens the way
the unrecorded case opens one today: an ordinary pull request that carries the
Stage below it until that one merges, with the Timeline saying so. That sentence
is already written where a Stage's base is decided; keep it true.

**A Stage cut from what is still the top rebases nothing.** That is every Stage
of every roadmap run in order, which is every roadmap until stage 04 lands, and
it is the whole reason this stage changes nothing anybody can see: the rebase
finds the branch already on the top and moves no commit.

**And the branch is never force-pushed after its pull request opens.** A Stage is
rebased once, before it has one. What moves a joined Stage afterwards is what
moves any stacked branch — the sync the repository's block already describes,
run after the default branch moves or a pull request below merges — and never a
later Stage joining.

The record of the join is already there from task 01: it is written where the
pull request is first recorded. What this adds in front of it is the rebase.

## Acceptance criteria

- [ ] A joining Stage's pull request has the Stage below it in the chain as its
      base.
- [ ] A Stage cut from what is still the top of the chain rebases nothing, and a
      roadmap run in order finishes exactly as it does today — same commits, same
      pull request, nothing extra on the Timeline but the join itself.
- [ ] The finish session is told what the chain is and that the rebase comes
      before the push, and it follows the target repository's own block where
      there is one.
- [ ] The branch is not force-pushed after its pull request opens, and no Stage
      already in the chain has its branch or its pull request rewritten by a
      later one joining.
- [ ] A repository recording no stacking mechanism still gets the rebase, and its
      pull request opens as the unrecorded case opens one today.
