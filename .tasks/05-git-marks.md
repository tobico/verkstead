# 05. Git marks

## What to build

The rows of the tree carry git's account of themselves, so what the agent
changed is visible before a diff is.

**One status read per root**, porcelain and NUL-separated so a path with
anything at all in its name survives, answered for the Conversation whole rather
than a call per row. Two marks, which is what a tree has any use for: a file git
sees as changed, and one it has never seen. A folder carries the strongest mark
of anything under it, so a change deep in a tree shows on the row above it
before anybody expands one.

**Read again on `files` and on `commit`.** The second is why it is its own
reading rather than a field on a folder listing: a commit clears every mark in
the Worktree without touching a file, so nothing about the tree has moved and
the marks have all changed. The reading is a query, so both kinds name it in the
nudge table.

**The colours.** Untracked is an addition and takes the Diff's own `--added`.
Modified takes a token of its own, defined in both schemes and seeded on the
accent ember — named for what it means rather than borrowing the Diff's red,
which means deleted, the way a stopped run was given its own name for the same
reason.

## Acceptance criteria

- [ ] A file saved in the pane is marked as changed, and every folder above it
      up to its root carries the mark.
- [ ] A file made in a terminal tab is marked as untracked rather than as
      changed, and the two are told apart at a glance.
- [ ] A commit made in a terminal tab clears the marks, with nothing in the tree
      having been written.
- [ ] A root git will not answer about draws its rows unmarked rather than
      failing to draw.
- [ ] The marks clear the contrast the palette holds itself to, in both light
      and dark.
