# 13. Drag-drop capture

## What to build

The conversation-list drag stops getting stuck when the pointer gets away
mid-drag. Today its move, up and cancel handlers live on the card element with
pointer capture, and nothing clears the drag state if the capture is lost or
the release never reaches the card — the list is left held and reordering.

The pane divider a few lines away already does this right: listeners go on the
window for the drag's lifetime, because a pointer that has outrun the element
is still dragging it. The list drag moves to the same pattern, and clearing on
lost capture as well, so every way a drag can end — release, cancel, lost
capture, the element re-rendering under the hand — drops the held state and
either commits or abandons the order cleanly.

The touch path keeps its long-press lift, its scroll refusal and its cleanup;
the trailing-click swallow and keyboard reordering stay as they are.

## Acceptance criteria

- [ ] Dragging out of the window and releasing there leaves nothing held; the
      next press starts a fresh drag
- [ ] A pointer cancel or lost capture mid-drag clears the drag state
- [ ] Ordinary reordering, tap-to-open, long-press on touch and keyboard
      reordering all behave exactly as before
