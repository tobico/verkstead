# 04. The sidebar item and its popup

## What to build

The conversations list's foot holds the *Show archived* switch alone today, a
sticky block at the bottom of the pane. After this task it reads **Forwarding 3
ports** to the right of the switch — *Forwarding 1 port* with one — drawn only
while this device has any Forward, forwarding or skipped, and not at all with
none. The foot stays the pane's last child, which is what its tests assert.

**A press opens a popup** of the device's Forwards, read from task 03's reading
and refreshed on the `forwards` Nudge, the way the list refreshes on its own.
Each row is a link opening `localhost` on that port in a new tab, and reads the
port, the device with its OS icon and name, and the Conversation's title under
them; a skipped row is dimmed and reads its reason, *port busy here*. The
popup hangs **upward** from the foot — the shared anchored popover only drops
downward today, and the foot is at the bottom of the pane — closes on Escape
and on a press outside, and hands focus back to the item. No row has a control:
nothing stops a Forward by hand, and the tab is the stop.

The item belongs with the switch wherever the switch is drawn in the list;
the compose page's copy of the switch, drawn only when the list is empty, draws
no item, there being no terminal anywhere to forward from.

## Acceptance criteria

- [ ] The item reads *Forwarding 1 port* and *Forwarding 3 ports* beside the switch, is absent at none, and the foot remains the pane's last child.
- [ ] The popup opens upward from the foot, lists each Forward as a link with port, device icon and name and Conversation title, dims a skipped one with its reason, and closes on Escape and outside with focus returned.
- [ ] A `forwards` Nudge refreshes the item and an open popup without a reload.
