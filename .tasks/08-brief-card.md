# 08. Brief card

## What to build

Tidy the Draft conversation's setup block:

- **Branch and base branch sit side-by-side**, the same way the Grilling and
  Implementation pairing pickers do — reuse that flex-wrap pattern (two
  columns that wrap to stacked when the pane is narrow), not a media query.
- **The branch hint Note goes entirely** — both its wordings: the pinned
  "Pinned to … — the work branches from wherever it stands when grilling
  starts." and the unpinned "The work branches from … as it stands when
  grilling starts."
- **The "Ready to grill." Note goes.** The readiness signal remains the
  Start grilling button's enabled state and its not-ready explanation, which
  stay as they are.

## Acceptance criteria

- [ ] Branch and Base branch render side-by-side in a wide pane and stack in
      a narrow one, matching the pairings' behaviour
- [ ] Neither hint wording nor "Ready to grill." appears anywhere
- [ ] Web tests pass, updated where they asserted the removed copy
