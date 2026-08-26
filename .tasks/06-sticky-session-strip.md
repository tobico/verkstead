# 06. Sticky session strip

## What to build

While an agent session is active, pin it to the bottom of the conversation
pane so it can always be found: a slim sticky strip carrying the session's
title and its activity mark (turning ring while working, idle circle while
idle). Tapping the strip opens the session's output in the details pane, the
same as tapping its timeline card. The session's card stays in its place on
the Timeline — the strip is a second appearance, not a move.

The strip shows only while the conversation has an active session and
disappears when the session ends. Mind the two scrolling regimes: from the
two-pane width upward each pane scrolls itself, below that the page scrolls —
the strip must hug the pane's bottom in both. Keep it beneath the sticky
top chrome in stacking terms.

## Acceptance criteria

- [ ] With a session running, the strip is visible at the pane's bottom
      however far the record is scrolled, on both narrow and wide layouts
- [ ] Tapping it opens the session output; its mark matches the session's
      working/idle state live
- [ ] No strip when no session is active
