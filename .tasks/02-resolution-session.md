# 02. Dispatch the resolution session

## What to build

When a poll reads a wrapping pull request as CONFLICTING and it has goes left,
dispatch a fix session at it, the way a red check gets one.

The session is the checks watcher's shape end to end: it runs under the
Implementation Pairing through the same dispatch the check fixes use, it takes
the Conversation's Turn before anything is counted (and simply comes back next
poll if the review or another session holds the Worktree), and it is dispatched
silently — the session's own commit is the record, and a Notice is written only
when the run stops. Its feedback names the pull request, the repository and the
worktree to work in, exactly as check feedback does, and tells it what to do:
merge the pull request's base branch into the work branch, resolve the
conflicts, run the repository's tests, commit and push. Merge, not rebase — no
force-push, nothing stacked on the branch breaks. (Task 03 makes the strategy
configurable; this task hard-codes merge.)

Extend the bundled addressing skill so conflict feedback is its fourth kind of
feedback, beside a failed check, a review finding and a comment — including
that the resolution merge commit is bookkeeping enough not to need a Commit
Summary fight, and that the session must not "resolve" a conflict by discarding
either side's work wholesale: a conflict is two changes to reconcile.

**Two goes per pull request**, counted as the session is dispatched, kept in
the store so a restart does not spend them again. A PR out of goes waits for
every other PR's goes the way a check out of goes does, and the stop's Notice
names the pull request that would not merge clean, with the tail of what the
last session said. Resume and a steer into Wrapping forget the count, exactly
as they forget the checks' fix attempts.

After the session pushes, nothing special: the push unsettles the checks, the
next poll reads mergeability afresh, and the wrap-up settles on its own rules.

## Acceptance criteria

- [ ] A CONFLICTING pull request with goes left gets one fix session told to
      merge the base branch in and push; a companion's PR gets one sent to the
      companion's worktree.
- [ ] Two goes per PR, persisted; when every red or conflicted PR is out of
      goes the run stops with a Notice naming the PR; Resume forgets the
      count.
- [ ] No session is dispatched while something else holds the Worktree, past
      a stop, or when the reading is UNKNOWN.
- [ ] The addressing skill documents conflict feedback as a kind of its own.
