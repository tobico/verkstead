# 08. Pinned carousel

## What to build

When a Conversation has more than one pinned card — task list, roadmap, pull
request — they stop stacking and pushing the timeline down. One card shows at
a time, in a carousel: dots beneath saying how many there are and which is
showing, arrows at the edges on pointer devices, swipe on touch. (A
kind-labelled tab row was offered and declined; the classic carousel is the
settled shape.)

Which card fronts when the conversation opens: the one needing attention —
a pull request with unresolved feedback, for instance — and otherwise a fixed
order: task list, roadmap, pull request. The position is not remembered
between visits.

The carousel lives where the stack lives now, inside the sticky chrome with
the pane header, so the visible card still travels with the header while the
timeline scrolls behind. A conversation with a single pinned card renders
exactly as today — no dots, no arrows. Cards keep their full behaviour
(opening the pull request pane, live task states) whichever is showing.

## Acceptance criteria

- [ ] With several pinned cards only one is visible; dots, edge arrows and
      touch swipe all move between them
- [ ] The attention-needing card fronts on open, else task list then roadmap
      then pull request; a single pinned card shows no carousel furniture
- [ ] The visible card keeps its sticky position and all of its existing
      behaviour
