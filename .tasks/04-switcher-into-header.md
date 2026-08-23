# 04. The switcher into the header

## What to build

Move the Transcript/Screen switcher off its own full-width row and into the
details pane's header, right of the "Agent output" title — the spot the Close
link holds today. **Close goes from this pane**: "← Timeline" remains the way
out, as it is on a narrow window already. The switcher shrinks to fit its two
labels instead of stretching across the pane.

Give the switch its motion: the accent-coloured active indicator **slides**
from the old segment to the new one — ease-in-out, **100ms** — instead of
jumping. The two segments keep their intrinsic widths, so the indicator
animates position and width both. The buttons themselves stay what they are:
two buttons saying which is pressed, with the indicator as presentation only,
so nothing changes for a screen reader. `prefers-reduced-motion` gets the
instant switch back.

The header wraps on a narrow pane as it does today; the switcher must land
somewhere sensible when it does rather than overflowing.

## Acceptance criteria

- [ ] The switcher sits in the header row beside the title, sized to its
      content, and the Close control is gone from the agent-output pane.
- [ ] Switching slides the active indicator between the two labels over
      100ms with ease-in-out; under reduced motion it moves instantly.
- [ ] Keyboard and screen-reader behaviour of the two buttons is unchanged
      (`aria-pressed` still says which is showing).
- [ ] On a narrow pane the header wraps without the switcher overflowing.
