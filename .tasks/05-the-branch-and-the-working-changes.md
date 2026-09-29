# 05. The branch and the working changes

## What to build

The git leg, for the Conversation's own repository. It runs after the record
has landed, because a Worktree hangs off a Conversation and the far end needs
one to hang it from.

The far end is asked what it already has — the tips it holds of the matched
Repo — and the bundle is packed against exactly that, so a machine that cloned
the same repository yesterday is sent the branch and not the history under it.
The bundle goes over the link, the far end fetches it, and cuts the Worktree on
the branch under its own Data Directory at a path of its own choosing. Nothing
that arrives is ever joined onto a directory there as a path: the far end names
its own, the way the memory sync does.

Beside the bundle go the working changes, and they are two different things.
Tracked changes travel as a **binary patch** — `git diff --binary` against
HEAD, which is what carries a changed image or a compiled fixture, and what
`diffs.rs` deliberately does not produce: the Diff a Question Set carries is
prose for a human, with binary left out and untracked files compared against a
path Windows does not have. So this needs a read of its own. Untracked files
git does not ignore travel as raw bytes, each by its path relative to the
Worktree. Ignored files stay behind, and the far end builds its own.

The line endings are already settled and cost nothing: a binary patch is
written in index form and `git apply` re-applies whatever working-tree
convention the target keeps, so a tree with `core.autocrlf` on and one with it
off exchange patches in both directions and land byte-identical content in each
machine's own convention. Untracked files are converted by neither end, which
is right for a file git is not tracking.

Which Worktrees a session can write in is a reading that already exists and is
reusable as it stands. And the branch is followed where a session renamed it —
what goes over is the name the checkout is actually on, not the name the record
was written with.

## Acceptance criteria

- [ ] The branch is at the same commit on both machines, the bundle packed
      against what the far end said it held rather than against nothing.
- [ ] Uncommitted tracked changes on the source are uncommitted tracked changes
      on the far end, a changed binary file included, and an untracked unignored
      file arrives with its bytes unchanged.
- [ ] A file the far end's own ignore rules cover is not carried, and the far
      end's Worktree sits under its own Data Directory at a path it named
      itself.
- [ ] A branch a session renamed on the source arrives under the name the
      checkout is on.
