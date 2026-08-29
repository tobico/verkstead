# 02. Extract the three-pane layout

## What to build

The workbench's three-pane frame becomes a shared component, with no change in
behaviour. Today the frame, the two draggable dividers, the per-device
remembered widths, the media-query layers (one pane, two beside, all three),
and the `data-pane` mobile walking all live inside the workbench page; the
settings page (a later task) needs the same frame, so what is the frame's
moves into a component both pages render.

The component owns: the grid and its `data-pane` attribute, which dividers
exist per layout, dragging/nudging/restoring them, and reading and writing
the remembered widths. The two pages share those remembered widths — one pair
per device, not one per page (settled in the grilling). What stands *in* each
pane stays the caller's: the workbench hands in the conversations pane, the
Timeline pane and the details pane exactly as it draws them today.

Which mobile level is showing remains driven by the caller (the workbench
derives it from the URL and its selection), so the component takes the level
rather than deciding it.

Pure refactor: after it, the workbench looks and acts exactly as before —
same widths remembered, same dividers, same phone walk.

## Acceptance criteria

- [ ] A shared layout component owns frame, dividers, widths and `data-pane`;
      the workbench renders through it
- [ ] Dragging, keyboard-nudging, double-click restore and the remembered
      widths behave exactly as before, and the stored widths carry over
- [ ] The phone's one-pane walk (conversations → timeline → details) is
      unchanged
