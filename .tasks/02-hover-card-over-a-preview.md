# 02. A hover card over a Preview's row, holding its command

## What to build

A Preview's row in the Code pane's Previews box says its title and nothing
else, and the human has no way to see what a press would type short of
pressing it. This task gives every row a **card** beside it holding the
command, so the row can be read before it is run. The card is drawn in the
chrome the Menu already has — the same wash, border, shadow and stand-off
from the page a context menu gets — and placed beside the row, over the
groups, where a card beside the side column has room.

For this task the card holds the command alone, as a code block. Task 03
puts the description above it, so build the card as a column that takes a
block above the code block without re-laying it out.

**How it opens and closes.** The viewer has no custom tooltip today — its two
hovers use the browser's own `title` attribute — so this is the first, and
it follows the pattern the sidebar already uses for a pointer resting on it
(*only a mouse arms it*):

- **A mouse resting on the row** opens the card, read off the pointer's type
  and not left to `:hover`, so a finger that tapped never holds it open; the
  pointer leaving the row closes it, with a moment's linger so a hand
  crossing to the card does not lose it. A pointer that merely crosses the
  row on its way elsewhere should not flash the card: wait for it to settle,
  the way the sidebar waits.
- **Keyboard focus** on the row opens the card; focus leaving closes it;
  Enter and Space run the Preview as a button's press does today.
- **Touch is two taps, on every row alike.** The first tap on a row opens
  its card and does not run the Preview; a second tap on the same row runs
  it; a tap anywhere else closes the card and runs nothing. The same on a row
  that will have no description once task 03 lands: the card holding the
  command alone is still worth a look, and a row that ran on the first tap
  while its neighbour did not would be two rules for one list.
- **Escape** closes an open card, as it closes a menu.
- One card at a time: opening another row's closes the first.

**The press itself is unchanged.** What a press does — decided by the server
against what is in front of the terminal — is not this task's; only when a
touch counts as a press is.

**Not on a phone layout's own terms.** The Code pane promises little under
40rem, where the box stacks above the tree; the two-tap rule holds there too
and the card may hang below the row rather than beside it, but nothing more
is owed.

The decision is recorded in the amendment at the end of ADR-0024 and in
CONTEXT.md under **Code**: build to those words.

## Acceptance criteria

- [ ] A mouse resting on a Preview's row shows a card holding its command as a code block, and the card closes once the pointer has left the row and the card.
- [ ] On touch, the first tap on a row shows its card without running the Preview, a second tap on the same row runs it, and a tap away closes the card; this is the same on every row.
- [ ] Keyboard focus shows the card, Enter runs the Preview, and Escape closes the card.
- [ ] The web suite covers hover, the two taps, the tap away and the keyboard, and the existing Previews tests still pass.
