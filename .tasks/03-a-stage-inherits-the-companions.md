# 03. A stage inherits the companion set

## What to build

A roadmap grilled with companions builds with them. The stage a settling
wrap-up starts carries its parent Conversation's companion set across and has
every one of them checked out before its first session runs.

**Through the one funnel that already copies the Pairings and the brief.** A
stage is given everything a human would have settled before pressing anything,
and the companions belong in that list for the same reason the Pairings do: a
stage has no draft moment of its own, so there is nowhere else the set could
come from. Every companion of the parent, in the mode it was in.

**Read-only ones come across as they are** — the same repository, the same base
— and are checked out detached at whatever that base resolves to for this
stage. What a stage reads is that repository as it stands when the stage starts,
which is the rule every other checkout Verkstead makes follows.

**Read-write ones cut a fresh branch per stage, named after the stage's own
branch.** Mirroring, whatever the parent's row said: a companion branch name
somebody typed while drafting is the roadmap Conversation's own, and two stages
sharing one companion branch would be two review units on one branch with two
pull requests fighting over it. So the typed name does not travel; the stage's
branch is what its companion branches are called.

**And where the stage's own branch stacks, its companion branches stack too.**
A stage stacks on its unmerged predecessor because that is where the work it
builds on is, and a read-write companion is in exactly that position: the
predecessor stage committed in it and its pull request there is unmerged for
just as long. So a stacked stage's companion branch is cut from the predecessor
stage's companion branch in that repository — which is that Conversation's
companion row resolved against that Conversation's branch — rather than from
the companion's configured base. An unstacked stage's comes off the configured
base, fetched and resolved as a grill start resolves one.

**The checkouts are made where the stage's own worktree is made**, in the same
act and recorded with it: a stage that said it was implementing with companions
nothing had checked out would be one nothing could bind into a sandbox and
nothing would come back and remove.

**A companion that cannot be delivered starts nothing.** Nobody is at a button
— this runs at the end of an unattended run — so a fetch git would not make, a
base that resolves to nothing or a companion branch already taken halts the
stage the way everything else that stops one halts it: a notice on the settled
Conversation's Timeline naming the repository and what git would not do, the
half-made stage Conversation closed, and nothing checked out left behind. A
stage that quietly built without a repository the roadmap was grilled against
is a worse outcome than a stage that waited.

## Acceptance criteria

- [ ] A stage of a roadmap whose Conversation was grilled with companions starts
      with every one of them checked out, bound at its own mode, and named in
      its sessions' prompts.
- [ ] Each stage's read-write companion branch is named after that stage's own
      branch, whatever the parent's row named, so no two stages of one roadmap
      share a companion branch.
- [ ] A stacked stage's read-write companion branch is cut from the predecessor
      stage's companion branch in that repository; an unstacked stage's comes
      off the companion's configured base, fetched and resolved at that moment.
- [ ] A companion that cannot be delivered starts no stage: a notice on the
      settled Conversation's Timeline names the repository, and no half-made
      stage Conversation and no stray checkout is left behind.
