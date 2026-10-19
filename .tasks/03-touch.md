# 03. Touch

## What to build

The collapsed pane on a screen with no hovering pointer — a phone, a tablet —
where *mouse over* has nothing to work with (Q2.2, Q16, Q17, Q18).

**A tap anywhere on the collapsed pane expands it and does nothing else.** The
logo, the new-conversation icon and the cards all stand in it, and none of them
acts on that first tap: the pane as a whole is the one target while it is
collapsed. The expansion is the same expansion a hover makes, with the same
shadow and transition, and the same width.

**A tap on a card in the expanded pane opens the conversation and collapses
the pane** (Q16), so what was opened is what the human sees. The
new-conversation link does the same: it navigates and the pane collapses. A
right-click's menu has no finger behind it; a long press on a card is already
how a card is lifted to be dragged, and a drag keeps the pane expanded for as
long as the finger is on it, as the hover does for a mouse — a pane that
collapsed mid-drag would be a drag of nothing.

**A tap elsewhere collapses it** (Q17): anywhere on the page outside the pane.
Nothing is washed behind the expanded pane — no backdrop, no dimming; the
shadow alone says it is over the pane beside it — so the pane the human taps
to collapse it is the pane they then see, and that tap goes where it was put.

**Decided per gesture by the pointer's type, not per device.** A laptop with a
touchscreen hovers with its mouse and taps with a finger, and gets both: a
mouse's press leaves the pane expanded until the pointer leaves (Q8), a
finger's collapses it. The sidebar already tells the two apart for the drag by
the pointer type that began the gesture, which is the reading to use.

The *with nothing open it stands expanded* rule from task 01 holds on touch
too: at the root the pane is open without a tap, and picking a card collapses
it.

Prove it in the vitest suite with pointer events of both types, and check it
by hand in a browser with touch emulation. Keep the sidebar's module docs true
to the gestures it now answers.

## Acceptance criteria

- [ ] A touch tap on the collapsed pane expands it and opens nothing; a second
      tap on a card opens that conversation and the pane collapses.
- [ ] A touch tap outside the expanded pane collapses it and lands on what was
      tapped, with no wash drawn behind the pane at any point.
- [ ] A mouse press on a card in the expanded pane opens it and leaves the pane
      expanded until the pointer leaves.
- [ ] A long press that lifts a card keeps the pane expanded for the length of
      the drag.
