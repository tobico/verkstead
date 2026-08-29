# 01. IconButton and the sidebar

## What to build

A new shared `IconButton` component, and the conversations sidebar rebuilt
around it: the ⋯ menu at the head of the pane goes away entirely, replaced by
what it held.

`IconButton` draws one Font Awesome icon as a pressable button, and carries
the concept of an open state the way `CardButton` does: it is another thing in
a pane that can be selected and opened into a subpane, so an open IconButton
is painted as the selected one, in step with how an open card reads. It takes
the icon, an accessible label (the icon says nothing on its own), a press
handler, and whether it is open. Pressing it while open does nothing new — no
toggle, matching `CardButton`'s metaphor.

Two changes to the sidebar use it:

- **The gear** (Font Awesome solid `gear`) stands where the ⋯ was, at the
  head of the conversations pane, and navigates to `/settings`. Its open
  state is true whenever the URL is under `/settings` — invisible until the
  pane is drawn on that page (a later task), but the component knows it now.
- **Show archived** moves out of the menu to the foot of the conversations
  list: anchored to the bottom of the screen when the list does not fill it,
  and simply after the list, behind the scroll, when it does (a footer pushed
  down by `margin-top: auto` in the pane's column, not a sticky overlay). It
  keeps the server-scoped switch semantics it has today — the choice is the
  human's across devices — and keeps its error line for a save that failed.

With both moved, the workbench-actions menu component is deleted. The ⋯ at
the head of a Conversation's Timeline is a different menu and stays.

## Acceptance criteria

- [ ] The conversations pane head shows a gear IconButton and no ⋯ menu; the
      gear opens `/settings` and reads as open while there
- [ ] Show archived sits at the foot of the conversations pane: on screen at
      the bottom for a short list, after the list behind the scroll for a
      long one, still flipping the server-side setting with its error line
- [ ] IconButton is a shared component (icon, label, press, open) ready for
      the plus buttons later tasks add
