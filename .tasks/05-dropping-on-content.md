# 05. Dropping a tab on a group's content

## What to build

**A group's content is a drop target too, and it has five zones.** The drag
built in task 04 gains them: while a tab is held over what a group is showing,
where the pointer is decides which of five things a release would do. The
centre **moves the tab into that group**, the way a drop on its bar does. Each
of the four edges **splits the group there** — right, left, below, above — with
the dragged tab alone in the new group and the rest of the group's tabs staying
where they were. That is the third of ADR-0019's three ways to make a split,
the other two having landed in task 02.

**The zone under the hand is highlighted**, so the human sees which of the five
they are in before they let go — the edges as the band the new group would
take, the centre as the whole of the content. One highlight at a time across
the whole pane: a pointer is in one zone of one group.

The rules already built hold here. The group the tab lands in becomes active; a
group whose last tab has just left disappears, so dragging a lone tab onto its
own group's centre changes nothing; and a terminal moved this way keeps its
socket and its scrollback exactly as one moved along a bar does.

**A split made by a drop nests like any other**, so an edge drop inside a group
that is already one side of a split adds a level rather than a sibling, and the
new group takes half of what the group it split had.

## Acceptance criteria

- [ ] A tab dropped on a group's right edge splits it right, with that tab
      alone in the new group.
- [ ] A tab dropped on a group's centre moves into that group without
      splitting, and makes it active.
- [ ] The zone under the pointer is highlighted while the hand is over it, and
      only one zone anywhere is highlighted at a time.
- [ ] An edge drop on a group that is already half of a split nests a new split
      inside it rather than adding a sibling.
