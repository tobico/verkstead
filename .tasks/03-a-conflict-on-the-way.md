# 03. A conflict on the way

## What to build

The conflict dispatch is today's, unchanged, and this task is the proof it runs
whole under the narrowed wrap-up — which is the thing this Process exists for,
so it is worth demonstrating rather than assuming.

Nothing about the dispatch moves. A pull request GitHub says conflicts has one
`addressing` session sent at it, told which pull request will not merge, which
worktree to work in, and what to do about it in the words of the **resolution
strategy** this repository is configured for — a merge or a rebase, each
saying the thing the other would have done. The go is counted as the session is
dispatched rather than as it ends, two per pull request, and the Turn is tried
rather than waited for. When both are spent and there is nothing left owed
anywhere, the run stops with a Notice naming the pull request and its base.

What is new is the surroundings: there is no review session and no batch
session competing for the Turn, so a resolution is dispatched on the first poll
that sees the conflict rather than queueing behind a review. Check that the
narrowing did not take anything the conflict path was standing on — the
mergeability reading that unsettles Mergeable, the goes' survival across a
restart, and the Resolve-conflicts press on a Fix Conversation that reached
Done and then conflicted weeks later, which forgets both counts and comes back
through the same narrowed wrap-up.

Write the tests this slice is; change the dispatch only where the narrowing
broke it.

## Acceptance criteria

- [ ] A Fix Merge Issues Conversation whose pull request conflicts gets exactly
      one `addressing` session, told the configured resolution strategy and the
      worktree to work in, with nothing else dispatched beside it.
- [ ] Two goes and then the stop, the Notice naming the pull request; a third
      poll dispatches nothing.
- [ ] A conflict resolved and pushed lets the Conversation reach Done once the
      checks go green and GitHub says it merges.
- [ ] The Resolve-conflicts press on such a Conversation sends it back through
      the narrowed wrap-up from no goes spent, with no review session run.
