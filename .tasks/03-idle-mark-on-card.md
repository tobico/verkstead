# 03. The idle mark on the sidebar card

## What to build

Carry the idle flag from task 02 onto the conversation list's entries, so the
sidebar card's working mark tells the same truth as the Timeline item: the
turning ring while the Conversation's session works, the empty circle while
it is running but idle. Today the card turns for hours while a grilling waits
on an ask, which is the spinner saying something is happening when nothing
is.

The card's precedence stands: the filled waiting mark — an ask left open, an
Interruption, a direction to choose — still outranks everything, so an idle
grilling with an open Set shows waiting, not idle. The idle circle appears
only where the card would otherwise show the working ring.

The mark is nothing to a screen reader, so the card's spoken label says idle
rather than working when the circle is what is drawn.

## Acceptance criteria

- [ ] A conversation whose session is quiet past the threshold shows the
      empty circle in the sidebar, and goes back to the turning ring when the
      session speaks — with no reload, via the same nudge task 02 announces.
- [ ] The waiting mark still wins over both working and idle.
- [ ] The card's screen-reader text distinguishes idle from working.
