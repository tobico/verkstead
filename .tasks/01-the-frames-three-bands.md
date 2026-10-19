# 01. The frame's three bands

## What to build

The frame's layout, redrawn around a sidebar that is collapsed until the
window is wide. Three bands replace today's three:

| Width | Conversations pane | Beside it |
|---|---|---|
| under 60rem | collapsed, 3rem | one pane: the timeline or the details, as the narrow walk names it |
| 60rem to 100rem | collapsed, 3rem | the timeline and the details, with the middle divider between them |
| 100rem and up | full, at its remembered share | the timeline and the details, both dividers |

**The frame fills the window at every width** (grilling Q13). Today under
60rem the page scrolls as a document and the pane the walk names is the whole
page; from 60rem the frame is the window's height, clips, and each pane scrolls
inside it. Now the second of those holds everywhere, so a phone's page never
scrolls and the collapsed pane stands the full height of the window beside the
one pane the walk shows. The sticky head and foot inside a pane already work in
a scrolling pane, which is the model they were written for.

**The collapsed column is 3rem** — `0.5rem` of padding either side of a `2rem`
item, in rem at a 16px root (Q6), which is the brief's 32px and 8px and grows
with the text size. The remembered sidebar share is read only in the full band;
in the two collapsed bands it is left exactly as the device settled it, the way
a width is already left alone while a layout that has no divider for it stands.
The sidebar's divider is drawn only in the full band. The middle divider is
drawn from 60rem, and its arithmetic has to know the column to its left is 3rem
rather than the sidebar's share — the floors module takes the frame's shape as
a value already, so what it is handed is the shape, not a guess.

**The frame with no list at all keeps its own 60rem line.** It has no
conversations pane and so nothing to collapse; nothing in it changes.

**Expanding** (Q3, Q7, Q10, Q8). While collapsed, a hovering pointer over the
pane or a keyboard focus inside it expands the pane to the width it would stand
at uncollapsed — the remembered share, clamped against this frame exactly as
the full band clamps it, which on a phone is the 16rem floor. It expands *over*
the panes beside it: the grid column stays 3rem and nothing to the right moves,
the pane is drawn on top with the `--lift` shadow the menus and toasts wear,
and it collapses back when the pointer leaves and nothing inside has focus. A
press on a card does not collapse it; the pointer leaving does. The width
changes in a transition of about 150ms, off under `prefers-reduced-motion`.
The hover rule goes under `@media (hover: hover)` like every other, which the
hovering test enforces.

**With nothing open it stands expanded** (Q14, Q20): at the root address, and
wherever else no conversation is open while the pane is collapsed, the pane is
drawn expanded over the empty paper until a card is picked. Below 60rem that
is what the walk's *conversations* level becomes for now; task 04 finishes the
walk.

**What the collapsed pane shows in this task is its ordinary content, clipped
at 3rem** with no ellipsis — the first characters of a title, the left edge of
the head. Task 02 draws it properly. What has to be right here is that the
pane's content is laid out at the *expanded* width and clipped, not laid out
at 3rem: every height has to be the height it has expanded (the brief), and
laying out at full width and clipping is what makes that true for free.

Maximising Code hides the collapsed pane as it hides the full one. The
toggle itself is drawn at every width from this task on (Set 1270 Q22), since
something now stands beside Code everywhere; what it hides under 60rem is the
collapsed pane.

Prove the bands with the vitest suite where it can be — which media query
holds, which dividers are drawn, which widths are read and written — and
measure the real layout in a browser at a phone width, 70rem and 100rem:
there is no Playwright here, so drive the system Firefox over marionette, as
the viewer's layout has been measured before. Keep the module docs of the
frame, its stylesheet and the widths module true to the three bands.

## Acceptance criteria

- [ ] At a phone width the collapsed pane stands 3rem wide beside the timeline
      for the whole window's height, the page itself does not scroll, and the
      timeline scrolls inside its pane.
- [ ] At 70rem three columns stand: the 3rem pane, the timeline and the
      details; the middle divider drags and is remembered, and no sidebar
      divider is drawn.
- [ ] At 100rem the sidebar is full at its remembered share, its divider drags,
      and a share dragged there is what the pane expands to at 70rem.
- [ ] Hovering the collapsed pane, or tabbing into it, expands it over the
      pane beside it with a shadow and without moving that pane; leaving
      collapses it; the width animates unless motion is reduced.
- [ ] At the root address with nothing open the pane stands expanded at every
      width under 100rem.
- [ ] The hovering test passes, and the frame with no list is unchanged at
      every width.
