# 04. A × per row

## What to build

The stop by hand (grilling of 2026-10-19, Q5, Q8 and Q10). The tab was the
stop, and it is not any more; the only other end is killing the server in a
terminal somebody has to open. ADR-0019 rejected a × per row for the first
build because the tab was the stop; that reason is gone.

**A press ends the Forward on this device and remembers the dismissal.**
Ending closes the listener and every connection crossing it, as every ending
does. The dismissal is a memory on the hub, keyed the way a Forward is — the
member, the Conversation, the terminal and the port — so that the next
reconcile, from a `ports` Nudge or the everything reading, does not retake
it. It is cleared when the port leaves the member's reading, so a server
stopped and started again is forwarded afresh, and when this device attaches
that terminal again, opening the tab being the plainest way to say *I want
this after all*. A × on a skipped row stops the retrying the same way. **The
row goes** rather than staying dimmed: opening the tab is the way back.

**A dismissal is memory**, like the Forwards themselves: a hub restart reads
everything the member offers and takes a dismissed Forward again, and a
second press is the answer. Keeping dismissals in the store was rejected.

**The endpoint is this device's own**, under the forwards namespace the hub
keeps back from the link, and it names the Forward it ends. A press on a
Forward the hub does not hold is nothing to do rather than a refusal.

**In the popup**, the × stands at the end of each row, forwarding, skipped and
unreachable alike, and the row is still one tap target for the link. The menu
closes the ways every menu does; a press on the × ends one Forward and leaves
the menu open with the row gone, so several can be ended in one visit. The
item at the foot stops being drawn when the last row goes, as it is not drawn
at none today. Follow the copy guide for the control's label.

## Acceptance criteria

- [ ] A press on the × ends the Forward: the port frees at the hub's loopback,
      the row is gone from the reading, and the member's next `ports` Nudge
      does not bring it back while the port is still offered.
- [ ] The server in the terminal stopping and listening again, or this device
      attaching the terminal again, takes the Forward again.
- [ ] A hub served afresh takes a dismissed Forward again.
- [ ] The popup draws a × on every row, and the viewer's test of the popup
      covers a press removing the row and the item going at none.
