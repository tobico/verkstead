# 02. The layout tree, and splitting a group

## What to build

**The pane draws from a layout tree.** A node is either a split — a direction,
and a share of the parent per child — or a group, which is a bar of tabs and
the one of them showing. The tree is the single thing the pane reads: what is
drawn, where the borders are, and which group is **active** are all read off
it. Splits nest to any depth. The shares are percentages of the parent for the
reason the frame's pane widths are, so a border settled on a laptop means the
same on a wider screen; they are not draggable yet, which is the next task.

**Three ways in, two of them here.** A group splits to the right or below
itself from **a tab's menu** — right-click on a tab, the way a sidebar card's
menu is reached, there being no tab menu at all today — and from **an icon at
the end of its bar**. The third way, a tab dragged to an edge, is tasks 04 and
05. The tab the split was made from goes on showing in both groups, as VS
Code's does; since task 01 that is one buffer under two views, so typing in one
shows in the other, the dot is on both tabs and one save clears both.

**A group whose last tab leaves disappears**, and its neighbour takes the room.
That is the whole of unsplitting — there is no command for it, by ADR-0019 — so
a nested group emptied collapses the split it stood in and the tree shrinks a
level.

**The active group is the one last pressed into**, and a file pressed in the
tree opens there. Everything already keyed by path or by terminal number —
the readings, the buffers, the titles, the sentences over a refused shell, the
saves in flight — stays pane-wide and untouched: what the tree replaces is the
flat list of tabs and the single record of which one is showing.

The register settling that runs on every opening of the pane now walks the tree
rather than a list: a terminal that has ended since is dropped from whichever
group holds it, and one the register has that no group does joins the active
group.

## Acceptance criteria

- [ ] Two splits nest, one inside the other, and each group draws its own bar
      and its own content.
- [ ] Closing the last tab of a nested group collapses it and the neighbour
      takes the room.
- [ ] The same file showing in two groups types together, wears the dot on both
      tabs, and one save clears both.
- [ ] A file pressed in the tree opens in the group last pressed into.
