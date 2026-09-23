# Groups and drags

The one group stage 01 built becomes any number. The pane draws from a **layout
tree** — split nodes with a direction and a share per child, groups of tabs at
the leaves — and a group splits beside or below itself from a tab's menu, from
an icon at the end of its bar, or by a tab dragged to one of its edges, any
depth, with dividers that drag between them. Tabs drag along a bar and into
another group's; files drag in from the tree; and a group whose last tab leaves
is gone, which is the whole of unsplitting. A file open in two groups is one
buffer seen twice — one text, one dot, one save. A reload comes back to the same
groups with the same tabs and the same unsaved text, and the shortcuts a browser
leaves alone move between tabs and open splits and terminals.

Everything here is ADR-0019's *Tabs and groups*, which is the rulebook the
stage is measured against. Stage 01 left the buffers inside the editors, so the
first task lifts them above the pane: a Monaco buffer is registered at the
file's own path and a second one there is refused, so two views of one file are
impossible until it moves. The tree's own row menu and quick open are stage 03,
and the watcher that follows the disk is stage 04.

Roadmap stage: [02: Groups and drags](docs/roadmaps/code-pane/02-groups-and-drags.md)

## Tasks

- [x] 01: One buffer behind every view — [details](01-one-buffer-behind-every-view.md)
- [x] 02: The layout tree, and splitting a group — [details](02-the-layout-tree.md)
- [x] 03: Dividers between groups — [details](03-dividers-between-groups.md)
- [x] 04: Dragging a tab along a bar and into another — [details](04-dragging-a-tab.md)
- [x] 05: Dropping a tab on a group's content — [details](05-dropping-on-content.md)
- [ ] 06: Dragging a file in from the tree — [details](06-dragging-from-the-tree.md)
- [ ] 07: The device remembers — [details](07-the-device-remembers.md)
- [ ] 08: The shortcuts — [details](08-the-shortcuts.md)
