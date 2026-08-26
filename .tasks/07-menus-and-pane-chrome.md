# 07. Menus and pane chrome

## What to build

Three chrome fixes settled by grilling:

- **One ⋯ menu trigger.** The conversations-list pane and the conversation
  pane each style their own menu trigger with deliberately duplicated CSS,
  and they read as slightly different sizes in context. Make the trigger's
  presentation part of the shared Menu component so both panes render the
  identical button, and remove the duplicated per-caller trigger styles.
- **Dropdowns take the page wash.** The Menu component's full-page backdrop
  is currently invisible by design; give it the same subtle darkening the
  answer-set navigation dropdown's backdrop has (a 20% black wash), so every
  dropdown menu dims the page behind it while open.
- **The Close link leaves the details panel.** Every details-pane view that
  offers a "Close" in its header loses it; the back link remains the way
  out. Remove the now-unused close wiring rather than leaving dead props.

## Acceptance criteria

- [ ] Both panes' ⋯ triggers come from one component and render identically
- [ ] Opening any dropdown menu washes the page behind it; clicking the wash
      closes the menu
- [ ] No details-pane view shows a Close link, and web tests pass
