# The Code pane

The Terminal pane becomes **Code**: a details pane at `/code` with a file tree
down its side over every Worktree the Conversation has, and one group of tabs
beside it holding files and terminals alike under a tab bar styled after VS
Code's. A file pressed in the tree opens in Monaco; Ctrl+S writes it back, and
a write over a file the agent has changed since is refused with a bar offering
Reload or Keep mine. Live shells come back as tabs; a New terminal button opens
another; × on a tab closes it, asking first where the file is dirty or the shell
is busy. A maximise toggle gives the pane the window. `/terminal` redirects, and
the Terminal pane is gone.

Everything after this stage stands on it: the groups that split, the tree's own
menu and quick open, and the watcher that follows the disk are stages 02, 03 and
04. What is built here is the one group, the one-folder-at-a-time tree, and the
reads and writes underneath both.

Roadmap stage: [01: The Code pane](docs/roadmaps/code-pane/01-code-pane.md)

## Tasks

- [x] 01: Code replaces the Terminal pane — [details](01-code-replaces-the-terminal-pane.md)
- [x] 02: The tab bar — [details](02-the-tab-bar.md)
- [x] 03: Busy shells — [details](03-busy-shells.md)
- [ ] 04: Roots and folders — [details](04-roots-and-folders.md)
- [ ] 05: Opening a file — [details](05-opening-a-file.md)
- [ ] 06: Monaco — [details](06-monaco.md)
- [ ] 07: Saving — [details](07-saving.md)
- [ ] 08: State above the pane, and maximise — [details](08-state-and-maximise.md)
