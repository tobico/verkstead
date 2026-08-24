# 01. The wrap-up session fixes what was approved, itself

## What to build

The wrap-up's review session no longer ends the moment it asks. It reviews the
branch, proposes its findings in one Question Set — Fix it here / Leave it per
finding — runs the ask as a background command, and when the answers come back
it fixes each accepted finding itself, honouring whatever the human wrote
beside their answers, commits, pushes, and ends. It holds the Worktree's Turn
across the whole wait, which is what keeps other sessions out from under it.

On the server side: the runner stops ending the review session on *asked*, and
answering the review's Set no longer dispatches addressing sessions — the
per-finding fix dispatch goes away. The Review settles as a thing wrap-up
waits on when the wrap-up session ends cleanly, not when the Response is
taken. A review that finds nothing still asks nothing, says so as its last
line, and settles.

The review block keeps travelling on the Set — the findings and the human's
answers are what a later safety net re-dispatches from — but answering it
dispatches nothing.

The reviewing skill is rewritten to match: it reviews, proposes, then fixes
what was accepted. Its rule that the session changes nothing goes; committing
and pushing the accepted fixes is now its job, following the addressing
skill's conventions for the commit messages.

## Acceptance criteria

- [ ] A finding accepted on the review's Set is fixed, committed and pushed by
      the same session that raised it, and no session is dispatched from the
      review's answers.
- [ ] The Review settles only when the wrap-up session ends cleanly; a review
      that finds nothing settles by saying so and asking nothing, as today.
- [ ] The reviewing skill proposes then fixes, carries the human's words beside
      each accepted finding into what it does, and leaves declined findings
      alone.
