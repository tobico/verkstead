# Following the disk

The tree and the editors follow the Worktree without being asked. An agent
writing a file, a build filling a folder, a terminal tab checking out a branch:
each reaches the page as a `files` Nudge from a watcher the server runs while a
Code pane is attached, and the page re-reads the folders it has expanded and the
files it has open. A clean editor takes the new text silently; a dirty one shows
the Reload / Keep mine bar the moment the disk moves rather than at the next
save.

And the rows carry git's account of themselves — modified, untracked, a folder
wearing the strongest mark of anything under it — read again on every `files`
Nudge and on every `commit`, since a commit clears the marks without touching a
file.

Roadmap stage: [04: Following the disk](docs/roadmaps/code-pane/04-following-the-disk.md)

## Tasks

- [x] 01: The kind and the attachment — [details](01-the-kind-and-the-attachment.md)
- [x] 02: The watcher walks the Worktree — [details](02-walking-the-worktree.md)
- [x] 03: The tree follows — [details](03-the-tree-follows.md)
- [x] 04: Editors follow — [details](04-editors-follow.md)
- [x] 05: Git marks — [details](05-git-marks.md)
