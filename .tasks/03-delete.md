# 03. Delete, and the tab that stays

## What to build

**An endpoint that takes a path away**, a folder with everything under it,
refusing in the body the way the three beside it do — a read-only root, a root
or a path that has gone, anything under `.git`, and a root itself, which is a
Worktree rather than something in one.

**One confirm before it**, drawn the way the app already asks about what cannot
be taken back — the same card a busy terminal and a dirty tab put up, naming
what is about to go and how much of it: a folder says it takes its contents
with it. One confirm for a folder rather than one per file.

**And the open tab stays.** A tab that vanished under somebody with unsaved
text would take the text with it, so a deleted file's tab is kept, read-only,
saying the file is gone, with whatever was in it still there to be read and
copied out. Ctrl+S in it writes nothing — there is nothing to write over. The ×
closes it the way it closes any other tab, and that is the only way it goes.

**And it survives a reload**, because the device is already holding the text of
every open file the disk has not got: a page that comes back finds the file
missing and draws the tab as gone rather than as a refusal to read, with the
remembered text in it. Which is the one thing that makes *copy it out later* a
promise rather than a hope.

The row goes from the tree, and the folder it was in is read again.

## Acceptance criteria

- [ ] A folder deletes with its contents after one confirm, and its row and
      every row under it are gone from the tree.
- [ ] A deleted open file's tab stays, read-only, saying the file is gone, with
      its text still there to copy out and Ctrl+S writing nothing.
- [ ] A reload comes back to that tab, still saying so, with the text still in
      it.
