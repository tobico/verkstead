# 04. Badges and quiet rows

## What to build

Two timeline restylings settled by grilling:

- **No more all-caps anywhere in the frontend.** Six badge styles use
  `text-transform: uppercase` today — the LIVE, DEFERRED and CLOSED Set
  badges, the BLOCKED ON YOU header badge, the diff per-file status pills,
  and the unreadable-Set badge. All become sentence case, and the
  letter-spacing that accompanied the caps is dropped with it.
- **Notices and manual-task rows take the card look.** Both are quiet
  left-rule lines today; restyle them to match the other timeline items'
  card treatment (the padded, bordered, rounded card surface). Keep their
  content and semantics; the Moved/Steered centered lines stay as they are.
  Preserve a visible selected state for a notice a "Blocked on you" jump
  points at, consistent with how other selected cards read.

## Acceptance criteria

- [ ] `grep -r "text-transform" web/src` finds no uppercase treatment
- [ ] Notice and manual-task events render as cards matching the other
      timeline items; Moved/Steered lines are unchanged
- [ ] Web tests pass, updated where they asserted the old styling
