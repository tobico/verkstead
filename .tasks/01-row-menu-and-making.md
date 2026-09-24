# 01. The row menu, and new file and new folder

## What to build

The menu a row of the tree drops, and the first two things on it.

**Two endpoints of the files API**, beside the roots, the folder listing and
the file the pane already reads and writes: one that makes an empty file under
a folder of a root, and one that makes a folder there. Both refuse in the body
rather than by status, the way every other answer of that API does and in its
words — a name that is already taken, a path under none of the Conversation's
Worktrees, anything under `.git`, a root that takes no writes, a Worktree that
has gone, a folder that has gone, and whatever the filesystem itself said.

**The menu is the one `ContextMenu` the app has**, opened by a right-click on a
row, with which row it is about held by the tree rather than by a menu per
row — which is exactly how the tab bar beside the tree already opens its own.
**The mouse's alone**: stage 02 made a long press on a file row the way it is
picked up and dragged into a group, so the gesture is spoken for, and the
sidebar's cards answer the same problem the same way. Code is desktop-first
and ADR-0019 promises nothing on a phone, so a touch screen gets no row menu
rather than a second gesture invented for one.

**New file** and **New folder** are offered on a folder row and on a root, and
on neither in a read-only root — the root's own flag, which the roots listing
already carries, so the rows are not drawn rather than drawn to be refused. A
file row offers neither.

**And the name is typed in place**: the row pressed expands, a new row appears
under it carrying nothing but a field, and what is typed there is the name.
Enter makes it, Escape takes the row away and leaves nothing behind, and a
refusal is the server's sentence drawn beside the field with what was typed
still in it. A modal was decided against — what is being named is a row in the
tree, and the tree is where it is seen.

Afterwards the folder is read again, there being no watcher until stage 04, and
**a new file opens as a tab in the active group** the way a file pressed in the
tree does. A new folder opens nothing.

## Acceptance criteria

- [ ] A right-click on a folder row offers New file and New folder; a name
      typed into the row's field and entered makes it, the folder redraws with
      the row in it, and a new file opens as a tab in the active group.
- [ ] A name that is already taken is refused with a sentence beside the field,
      the field keeps what was typed, and nothing is written to the disk.
- [ ] A read-only root offers neither row anywhere under it, and both endpoints
      refuse a path in one on their own account.
- [ ] Escape while naming leaves the tree exactly as it was, and a right-click
      on a file row offers neither row.
