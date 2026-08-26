# 01. Cards and marks

## What to build

Three fixes to how sidebar conversation cards signal state, all settled by
grilling:

- **Attention** keeps only the filled accent disc. The "!" glyph inside it
  goes, and so does the waiting card's accent border and inset ring — the disc
  alone says it. The combined selected+waiting stripe treatment goes with
  them. Screen-reader wording is unaffected.
- **Selection wins over every other border treatment.** A selected card's
  border is solid accent at full visual strength, overriding the Draft card's
  dotted style and not dimmed by the Done/Closed card's faded look — dim the
  card's contents rather than its border where the two meet.
- **The idle session ring takes the accent.** A running-but-idle session's
  hollow circle indicator (in the sidebar mark and on timeline rows) is drawn
  in the ember accent color instead of grey. The turning ring of an active
  session stays as it is.

## Acceptance criteria

- [ ] A waiting card shows the filled disc with no glyph, and its border is
      the ordinary card border
- [ ] A selected Draft card shows a solid accent border; a selected Done or
      Closed card's accent border reads at full strength while its contents
      stay dimmed
- [ ] The idle indicator circle is accent-colored in both light and dark
      themes; web tests still pass
