# 05. Resume rebuilds a broken worktree

## What to build

Before recomputing, Resume validates the Conversation's worktree: the
directory exists, git answers inside it, and it is a registered worktree on
the Conversation's branch. A worktree that fails — deleted, hollowed out, or
dropped from the repository's worktree list, which is precisely what has
Conversation 15 stuck — is rebuilt from the branch and driving carries on.

A worktree is derived state: the branch holds everything committed, so
rebuilding loses nothing git could still report. Where git *does* answer and
shows uncommitted changes, the worktree is not broken and no rebuild
happens — validation touches nothing healthy.

## Acceptance criteria

- [ ] Resume on a Conversation whose worktree directory is gone, or whose
      registration git no longer knows, rebuilds it on the right branch and
      starts the next step.
- [ ] A healthy worktree — including one with uncommitted changes — passes
      validation untouched.
- [ ] A rebuild that fails refuses by name rather than halting silently.
