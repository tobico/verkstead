# 04. PR comments and settling

## What to build

The last of what wrap-up watches, and the rule that ends it.

**Comments.** While a Conversation is Wrapping, Verkstead polls its PR's
comments through the host `gh`. Comments it has not seen before dispatch an
addressing session — task 02's bundled skill again, given the comments as its
feedback. One session per batch rather than one per comment: a human writing
three replies in a minute is making one point, and three sessions racing each
other in one Worktree is the thing a batch prevents.

Which comments have been dispatched for has to survive a restart. A server that
came back up and read every comment as new would dispatch a session for
feedback that was addressed yesterday, so what has been seen is recorded rather
than held in memory.

**Settling.** A Conversation leaves Wrapping for **Done** when three things are
true together: the checks are green, the review's Question Set has been
answered, and no comment is left unaddressed. Verkstead decides that itself —
there is nobody at the workbench to press anything, which is the whole of what
running unattended means.

What it does *not* wait for is the merge. Stages stack on unmerged
predecessors, so a Conversation that stayed in Wrapping until its PR landed
would hold up every stage behind it, and merging is the human act this pipeline
is built around rather than a step in it. Done means Verkstead has finished with
the work, not that it is on `main`.

Any one of the three missing keeps it in Wrapping. A fix session dispatched by
any of the three unsettles what it was dispatched for: a commit pushed to the
PR is a new CI run to wait on.

## Acceptance criteria

- [ ] New comments on the PR dispatch one addressing session between them, not
      one each.
- [ ] Comments already dispatched for are not dispatched again after a restart.
- [ ] A Conversation whose checks are green, review Set answered and comments
      addressed moves to Done, with the move on the Timeline.
- [ ] Missing any one of the three keeps it in Wrapping.
- [ ] A commit landing on the PR puts the checks back to waiting rather than
      leaving them settled from the previous run.
- [ ] Nothing waits on the PR being merged.
