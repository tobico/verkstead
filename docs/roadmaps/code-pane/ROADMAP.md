# Code pane roadmap

The Terminal pane grows into **Code**: a file tree over every Worktree a
Conversation has, files edited in Monaco, and terminals as tabs beside them in
groups that split — a desktop place for trying the agent's work and finishing
it by hand without leaving the workbench. The decisions and their why are in
[ADR-0019](../../adr/0019-the-code-pane.md), which amends the tab rules of
[ADR-0013](../../adr/0013-conversation-terminals.md); the terms are in
[CONTEXT.md](../../../CONTEXT.md) under **Code** and **Terminal**. The briefs
reference both rather than restating them.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

Stage 01 is the pane itself and everything after it stands on it. Stages 02,
03 and 04 each depend on 01 alone and are reorderable among themselves: the
groups need nothing from the tree's menu or the watcher, the menu needs nothing
from the groups, and the watcher's one tie to the others is that the reload bar
it makes earlier is the bar stage 01 already draws on a refused save.

## Stages

- [x] 01: The Code pane — [brief](01-code-pane.md)
- [x] 02: Groups and drags — [brief](02-groups-and-drags.md)
- [x] 03: Working the tree — [brief](03-working-the-tree.md)
- [ ] 04: Following the disk — [brief](04-following-the-disk.md) *(in progress: `code-pane/04-following-the-disk`)*
