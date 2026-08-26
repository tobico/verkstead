# 09. Hover sweep

## What to build

Give every interactive element a hover state, following the quiet scheme
settled by grilling, and gate all of it behind `@media (hover: hover)` so
touch devices never see it:

- **Surfaces** — clickable cards (sidebar conversations, timeline items,
  option rows), menu rows, filled buttons — shift their background one step
  (the paper/card swap the menu rows already use).
- **Links and quiet buttons** — text links, the quiet action button, pane
  back links, the menu trigger — take the accent color.
- **Text fields** — inputs and textareas — darken their border on hover, and
  gain a focus border while at it (none exists today).

Keep it consistent with the few hover states that already exist rather than
inventing new effects: no transforms, shadows, or transitions. Pair
`:focus-visible` with hover where an element lacks any focus treatment.

## Acceptance criteria

- [ ] Every `cursor: pointer` element in web/src has a hover treatment from
      the scheme above, none outside the hover media query
- [ ] Touch devices (hover: none) get no hover styling
- [ ] Keyboard focus is visible on the elements the sweep touched
