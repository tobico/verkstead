# 04. Dragging a tab along a bar and into another

## What to build

**A tab is picked up and put down with the pointer**, the way a sidebar card
is: the pointer captured at the press, the rest of the gesture watched at the
window so a hand that has outrun the tab is still dragging it, a grace distance
before a press becomes a drag, a hold rather than a distance to lift one under
a finger, and every way a gesture can end — the release, a cancel, the pane
going away mid-drag — putting the tab down. That is the pattern to follow; the
attachments' own drop is not, being the browser's `DataTransfer` for files
coming from outside the page.

**Where it lands is an insertion point along a tab bar** — its own bar, or any
other group's — marked while the hand is over it, so the human can see where
the tab will go before they let go. Dropping reorders it within its bar, or
takes it out of one group and into another at that point; the group it lands in
becomes the active one, and a group whose last tab has just left disappears,
which is the rule task 02 already built.

**A drag that returns to where it started changes nothing**, and neither does
one released away from any bar — this task's drop targets are the bars alone,
and a group's content becomes one in task 05.

**A terminal moved keeps its socket.** The tab carries a live connection and a
grid with what has scrolled past it in the window's own memory; a move that
remade the tab would close the socket, throw the scrollback away and reattach
to a repaint. So the thing moved through the tree has to keep its identity, and
the test for it is a terminal dragged into another group still showing what it
showed and still taking typing.

## Acceptance criteria

- [ ] A tab dragged along its own bar lands where the marker stood, and one
      dropped where it was picked up changes nothing.
- [ ] A tab dragged onto another group's bar leaves its group, lands at the
      marked point, and makes that group active.
- [ ] A terminal tab moved between groups keeps its socket, its scrollback and
      its typing.
- [ ] Dragging the last tab out of a group collapses that group.
