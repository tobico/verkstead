# 05. Following a rename

## What to build

A session may rename its Conversation's branch in its own Worktree with git,
and Verkstead expects it rather than repairing it. The reading that tells a
rename from a breakage: a Worktree whose recorded branch **no longer exists**
and whose HEAD sits on another branch has been renamed — the record follows to
the new name. A Worktree whose recorded branch **still exists** while HEAD is
elsewhere, or one on a detached HEAD, is broken and rebuilds exactly as today.

Following means every reader agrees promptly: the health check stops treating
the renamed checkout as unhealthy, the commit sweep finds commits on the new
name and lands them on the Timeline, and whatever else consults the recorded
branch — resume, stacking, the pull request — sees the followed name. In the
same act, each read-write Companion Repo left on the empty *mirroring* setting
has its branch renamed to match, so the mirror rule never resolves to a name
no companion branch actually has; a companion branch the human named is
untouched. The Worktree directory keeps the name it was made with — it is
cosmetic, and moving a live worktree is another way to fail.

## Acceptance criteria

- [ ] After `git branch -m` in a live Worktree, the record follows: commits
      on the new name reach the Timeline and the health check does not
      rebuild the checkout.
- [ ] Mirroring companion branches are renamed along; a companion branch the
      human named keeps its name.
- [ ] A mismatch with the recorded branch still standing, or a detached HEAD,
      still reads as broken and rebuilds; the Worktree directory name is
      unchanged either way.
