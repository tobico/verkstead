# 04. The ending reads the branch

## What to build

The human ticks **Nothing else** and the session signals, and what a Tinker does
next is decided by asking git what stands on the branch past the commit it was
cut from. Asked once, at the ending, and asked of the repository rather than of
the record.

- **Commits on the branch** means Wrapping. The branch is on no pull request, so
  the ending takes the path the run already has for exactly that: a session of
  its own on the `submitting` skill, sent to push and open one, and then the
  ordinary wrap-up over what it opened — reviewed where a Review Pairing was
  picked, skipped where it was picked away, with the checks and the comments
  watched as always.
- **No commits** means Done, with the move on the Timeline and nothing
  dispatched: there is nothing to open a pull request over and nothing to wrap
  up. The Worktree stays as it is for any Done Conversation.

**Not the follow-up's `pushed` flag, and reusing it would be a bug.** `pushed` is
worked out as *more commits than when this session launched*, and a relaunched
follow-up reads that baseline again — so a Tinker that commits, loses its
session, is relaunched and then ends on a round that committed nothing reads
`pushed: false`, and would land Done with work on a branch nothing is watching.
The branch is asked instead, and `pushed` goes on meaning what it has always
meant: whether the checks of a wrap-up already under way have to be waited on
again.

**The store learns a second landing out of Follow-up**, the way the start press
has two: the move to Wrapping it has today, and a move to Done beside it, each
with its own line on the Timeline. And the pull request record learns one more
door — a Conversation in Follow-up is one a pull request can move into Wrapping,
alongside the states that already move. That is what lets the `submitting`
entry land a Tinker without a second wrap-up entry written beside it.

**A follow-up steered into on a pull request is untouched.** It is not a Tinker,
its branch is already on a pull request, and where it ends is where it started:
back in the wrap-up it was opened over, with the checks re-waited on where it
pushed.

## Acceptance criteria

- [ ] A Tinker that committed and was marked **Nothing else** lands in Wrapping
      with a pull request opened and recorded against it, and the wrap-up's
      watchers running over it.
- [ ] A Tinker that committed in an earlier session and nothing at all in the
      session that ended lands in Wrapping too.
- [ ] A Tinker that never committed lands Done, with the move on its Timeline, no
      session dispatched and no pull request asked for.
- [ ] A follow-up steered into on a pull request ends in its wrap-up exactly as
      it does today, its checks re-waited on where the round pushed and left
      settled where it did not.
