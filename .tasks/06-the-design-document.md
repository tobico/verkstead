# 06. The design document

## What to build

The decisions the five tasks built, written where the project keeps them, so
the next session reads them off the branch rather than off this backlog.

**The design document's UI section** gains a settled paragraph beside the one
on the resizable panes, dated to the grilling of 2026-10-19 and the building of
this branch: the three bands and their widths; that the frame fills the window
at every width now, replacing the phone's document scroll; the collapsed
pane's 3rem and what it draws; what expands it — hover, focus, a tap — and
what collapses it, by pointer type; that with nothing open it stands expanded;
that the timeline's and settings' way back to the conversations is gone; the
Mac head dropping below the lights; and that width alone decides, with no pin
and no manual collapse. Name what was considered and rejected so it is not
reopened: collapsing only in the two-pane band, hover-only on touch, a fixed
16rem expansion, a hover delay, and a half-collapsed state. The paragraph on
the resizable panes says *below the two-pane breakpoint the layout pages one
pane at a time* and *the sidebar's alone with two*; both are no longer true
and are revised in place, in the document's own *revised* style.

**The glossary** gains an entry under the workbench section for the sidebar's
two states — **Collapsed** and **Expanded**, or one entry naming both — saying
what the collapsed pane is, when it stands, and what expands it, with *rail*,
*drawer*, *mini sidebar* and *hamburger* under Avoid. The entries that
describe the sidebar's foot and the walk through the panes are read for
anything the bands made untrue.

**The copy guide** needs nothing unless a task added a string it does not
cover; check the *New conversation* label and tooltip against it.

No ADR: the frame's bands are a design decision the design document already
owns, and no architecture changed hands.

## Acceptance criteria

- [ ] The design document's UI section describes the three bands, the
      collapsed pane, what expands it and the rejected alternatives, dated, and
      no sentence in it still says the phone pages one pane at a time or that
      the sidebar's divider stands with two panes.
- [ ] The glossary names the collapsed and expanded sidebar with the words to
      avoid, and no other entry contradicts the bands.
- [ ] Every string the five tasks put on a screen follows the copy guide.
