# 14. Context menu on cards

## What to build

Right-clicking a conversation card in the sidebar opens that conversation's
actions menu as a context menu, in place — exactly the rows the conversation
pane's ⋯ menu would offer that conversation in its current state (stop,
close, archive/unarchive, and whatever else the state admits), acting on the
right-clicked conversation whether or not it is the open one.

This is a pointer affordance: touch devices keep long-press for dragging
(task 02) and are untouched here. Reuse the shared Menu component and the
actions the pane menu already has rather than duplicating either; the menu's
page wash and styling come along for free.

## Acceptance criteria

- [ ] Right-click on any card opens the menu at the pointer with the same
      rows the pane menu would show for that conversation
- [ ] Actions taken from it affect the right-clicked conversation, not the
      open one
- [ ] Touch behaviour is unchanged
