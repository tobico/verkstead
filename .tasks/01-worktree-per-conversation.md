# 01. Worktree per Conversation

## What to build

Pressing *start grilling* on a drafting Conversation gives it somewhere to
work: a branch off its base commit and a worktree of its Repo, created on
disk, with the Conversation moving from `Draft` to `Grilling` and the move
landing on the Timeline. No session runs yet — this task is the ground a
session will later stand on.

The Brief freezes here, which is what the existing drafting refusals already
describe: past `Draft`, the Brief and the branch name stop being the human's to
change. That behaviour exists; this is the first thing to actually trip it.

Starting has preconditions worth refusing clearly rather than discovering
halfway: a Conversation needs both Profiles chosen, a Brief with something in
it, and a Repo whose base commit still resolves. Each refusal names itself, the
way the Repo registration outcomes already do.

The other half is undoing it. A Conversation gains an **abort** action, which
is what worktree teardown hangs off — the stage brief assumed an archive action
that does not exist. Aborting removes the worktree and leaves the branch alone:
a branch is cheap and may hold work worth reading, while a worktree is a
directory the human did not ask to keep. Aborting is reachable from any state
this stage can reach.

Where worktrees live is the service's state directory, beside the database,
because that is the one place the packaged unit is given to write.

## Acceptance criteria

- [ ] *Start grilling* in the workbench creates the branch and worktree, and the
      Conversation shows as `Grilling`
- [ ] The branch is created off the Conversation's base commit, in the Repo's
      own git directory, and the worktree is registered with it
- [ ] Starting is refused, by name, when a Profile is unchosen, the Brief is
      empty, or the base commit no longer resolves
- [ ] Editing the Brief or the branch name is refused once grilling has started
- [ ] Aborting a Conversation removes its worktree and leaves its branch in
      place; aborting twice is not an error
- [ ] A Conversation whose worktree has been removed underneath it says so
      rather than failing obscurely
