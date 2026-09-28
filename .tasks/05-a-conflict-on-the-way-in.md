# 05. A conflict on the way in

## What to build

What the joining session does when the rebase onto the top of the chain
conflicts: **it resolves it**, runs the repository's checks again over the
result, and asks the human only where it cannot tell which side is right.

The decision is that a conflict at the join is the joining session's to resolve
rather than something to stop on — always stopping to ask was weighed and turned
down. So this is words in the skill the finish session runs, and the words have to
carry three things:

- **Resolving it is the job.** A conflict is two changes to reconcile, and taking
  one side's hunk wholesale throws away work somebody did. The same sentence is
  already said to a session sent to sync a stack; say it the same way here.
- **The checks run again afterwards.** A rebase that conflicted produced a tree
  neither branch had, so the work is not built until the repository's own checks
  have been over it. Only then is anything pushed.
- **Ask only where it cannot judge.** Where the two changes are two intentions
  and the session cannot tell which is meant, that is a Question Set through the
  ordinary ask rather than a guess committed — one Set, saying what conflicted
  and what each side was doing, with the options it can see.

And the Timeline says the join met a conflict either way, because a join that
took a session an hour and a rebase that moved nothing should not read alike.

## Acceptance criteria

- [ ] A conflict the session can resolve ends in a resolved rebase, green checks
      and a pull request, with nothing asked.
- [ ] A conflict it cannot judge ends in a Question Set naming what conflicted,
      rather than a guess committed or a run stopped with nothing said.
- [ ] The Timeline says the join met a conflict, whichever of the two happened.
- [ ] Nothing below the joining branch is touched while the conflict is resolved.
