# 02. Rename, and the tabs that follow it

## What to build

**An endpoint that moves a path within its root**, and never across two: two
roots are two repositories, and a rename that crossed them would be a file
taken out of one checkout and put in another. What the human types is a *name*
rather than a path, so crossing roots is not a thing the pane can be asked to
do. It refuses in the body like the two endpoints beside it — a name already
taken, a read-only root, a root or a path that has gone, anything under `.git`,
and a root itself, which cannot be renamed because it is a Worktree rather than
something in one.

**A Rename row on the menu** of any row but a root, and the name typed in place
again: the field is drawn over the row it is about, holding the current name,
and Enter moves it. The row's own field rather than a modal, as the new row's
is.

**And every open tab follows its file**, which is most of this task. A renamed
file's tab is retitled, its reading is re-keyed, and its buffer is re-made at
the new path — a Monaco model is registered at the file's own address and the
package cannot rename one, so the model is disposed and made again with the
same text in it. What carries over is the text and whether it is dirty; what
goes is the undo stack, which is Monaco's limit rather than a choice. The next
Ctrl+S writes to the new path over the version the rename left behind. The same
file open in two groups is one buffer under both, so both tabs follow together.

The path is written down in more places than the tab: what the pane has open,
which folders of the tree are open, the bar a refused save left standing, and
the text this device is holding for a file it has not saved. A folder renamed
carries everything under it — every open tab whose path was inside it, and
every folder of the tree expanded under it.

Afterwards the folder is read again, as it is for a file made.

## Acceptance criteria

- [ ] A renamed open file's tab is retitled, its unsaved text is still there
      and still dirty, and the next Ctrl+S writes to the new path.
- [ ] A folder renamed carries its children's tabs and the folders open
      beneath it, and the tree redraws under the new name.
- [ ] A name already taken is refused with a sentence and nothing moves on the
      disk; a root offers no Rename row, and no field can name a path in
      another root.
