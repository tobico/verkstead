# 01. The composer pane

## What to build

Move a drafting round's editing surface — the brief field, the setup
controls and the start button — off the Timeline and into a new details pane,
reached the way every other details pane is. The Timeline's Brief card stops
being a field: it shows the server's rendering clamped to five lines at all
times, drafting or frozen, and while its round drafts, pressing it opens the
new pane. Opening a Draft conversation lands on the brief pane rather than
the timeline.

The pane serves conversations *while they draft*, whatever the brief's own
freeze: an adopting draft (whose only brief arrives frozen) gets the pane
with the box locked to the frozen rendering and the Adoption control where
the start button goes; a later round opened by a steer gets it with the
branch and base already frozen, pairings live. Once the conversation is past
drafting, a frozen brief keeps opening the existing read-only facts pane —
that pane is untouched.

Behaviour carries over unchanged in this task: the brief field keeps itself
on the same settling rules with no Save and no word about saving, every setup
field keeps its own immediate save and named refusals, and the start button
keeps its readiness verdict and hover explanation. The timeline foot loses
the button — the pane is its only home. Restyling is task 02; this task is
the move.

## Acceptance criteria

- [ ] A drafting brief's Timeline card is the five-line clamped rendering,
      never a field, and pressing it opens the composer pane; the pane is
      addressable by URL like every details pane.
- [ ] The composer pane holds the brief field, the full setup (branch, base,
      companions, three pairings) and the start button, each behaving as it
      did on the card; the timeline foot no longer draws the button.
- [ ] Opening a Draft conversation lands on the brief pane; past drafting, a
      frozen brief still opens the read-only facts pane.
- [ ] An adopting draft's pane locks the box to the frozen rendering and
      carries the Adoption control; a later drafting round's pane omits what
      is already frozen, as the setup card does today.
