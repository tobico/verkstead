# 02. Whole-card dragging

## What to build

Remove the ⋮⋮ drag handle from sidebar conversation cards and make the whole
card the drag target for reordering, without breaking clicking or touch
scrolling. Settled behaviour:

- **Mouse**: pressing anywhere on the card and moving past a small grace
  threshold (a handful of pixels) starts a drag; releasing inside the
  threshold is a click and opens the conversation.
- **Touch**: a long-press lifts the card, after which dragging reorders; a
  plain tap opens, and swiping scrolls the list as it always did. Do not put
  `touch-action: none` on the whole card — that is what would kill scrolling;
  suppress scrolling only once a lift has happened.
- **Keyboard** reordering (arrow keys on the row) stays as it is.

The existing hand-rolled pointer-event reordering and its server persistence
stay; only the initiation changes. Remove the handle's styles and the grip
glyph with it.

## Acceptance criteria

- [ ] The handle is gone; cards reorder by mouse drag from anywhere on the
      card, and a plain click still opens
- [ ] On touch, tap opens, swipe scrolls, and long-press then drag reorders
- [ ] Keyboard reordering still works and the order still persists across
      reload
