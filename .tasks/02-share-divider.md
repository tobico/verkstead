# 02. A divider between the share's two panes

## What to build

The share's two panes — the Timeline and whatever it has open — stand side by
side on a wide window with no divider between them, where the workbench's panes
all have one. Give the two-pane frame the same resizing the three-pane frame
has, so a reader can trade room between the record and the pane it opened.

The frame (`Panes`) currently draws dividers only where a conversations pane
exists; the two-pane share deliberately got none. Extend the frame and the
widths model so the two-pane layout has one divider, between the middle pane
and the details, with the full behaviour the app's dividers have:

- Drag with a pointer; arrow keys move a focused divider; double-click restores
  the default.
- Each pane keeps its minimum width in `rem` (the existing floors: 24rem for
  the middle pane, 24rem for the details), met against the frame as it stands
  and re-met when the window changes shape.
- The settled width is remembered per device, the way the app's widths are —
  and where the context has no storage (some `file://` openings, the viewer's
  sandbox), the drag still works and the width simply lasts for the tab. The
  existing storage guards already swallow refusals; nothing may throw.

The divider exists only where the two panes stand side by side (the same 60rem
breakpoint the app uses). A narrow window still walks one pane at a time, with
no divider and nothing remembered being read — unchanged from today.

The two-pane width should not rewrite what the device remembers about the
workbench's three-pane frame: the share is its own document with storage of its
own in practice, but the model must stay correct if the two ever meet.

## Acceptance criteria

- [ ] In a downloaded share at a wide window, the divider between Timeline and
      details drags, nudges with arrow keys, and double-click restores the
      default — with both panes held to their minimum widths.
- [ ] The settled width survives closing and reopening the share where the
      browser allows storage; where storage is refused the page works
      unchanged.
- [ ] Below the side-by-side breakpoint the share still shows one pane at a
      time with no divider.
- [ ] Web tests cover the two-pane divider's travel and floors alongside the
      existing three-pane tests.
