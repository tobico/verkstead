# 05. Carousel

## What to build

Three fixes to the pinned-cards carousel at the top of the conversation pane:

- **The arrow buttons stop overlapping card content.** Give the cards side
  padding so their content clears the arrows — only where the arrows exist,
  which is hover-capable screens (they are already hidden behind the hover
  media query); touch screens keep the full width.
- **The dots move above the card** instead of below it, so they hold still
  when the cards' heights differ.
- **A slide animation between cards**, roughly 200ms, direction-aware, and
  gated on `prefers-reduced-motion: no-preference` the way the existing tab
  indicator animation is. The carousel currently mounts one card at a time,
  so the transition will need both the leaving and the entering card in the
  DOM while it runs; swipe and arrow navigation both animate.

## Acceptance criteria

- [ ] With a pointer, arrows never sit over card text; on touch the cards
      keep full width
- [ ] Dots render above the card and stay put when switching between cards
      of different heights
- [ ] Moving between cards slides in the direction of travel, and does not
      animate under reduced motion
