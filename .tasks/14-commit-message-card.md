# 14. Commit message card

## What to build

The commit pane's Message card becomes the Preface card, actually shared. The
two are the same box in copied CSS today, but the Message caps itself at the
prose measure and its pane never declares the gutter variables — so it sits
narrow, and a wide diagram in a commit message cannot bleed out the way it
does in a Preface.

- Extract one shared card component for the two, so the box cannot drift
  again.
- Wire the commit pane's gutter and bleed the way the Set page's main declares
  them, so the Message card spans the column with the hanging margin and wide
  content — diagrams above all — bleeds back across it identically in both
  places.
- Move the contents navigation above the Message block, so the sidebar starts
  immediately below the pane header instead of level with the diff.

## Acceptance criteria

- [ ] Preface and Message render from one shared component and are visually
      identical: full width, gutter, wide-content bleed
- [ ] A diagram in a commit message takes the whole card, as it does in a
      Preface
- [ ] The commit pane's navigation sidebar starts under the header, with its
      jump targets still working
