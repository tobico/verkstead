# 10. Pane sizing

## What to build

The workbench panes become resizable on desktop. The underlying widths
change from fixed rem values to **percentages**, and the borders between
panes drag: both dividers in the three-pane layout, and the single sidebar
divider in the two-pane layout. Sizes persist **per device**, beside the
existing per-device settings, with sensible minimum widths so no pane can be
dragged useless, and a **double-click on a divider restores the defaults**.

Below the two-pane breakpoint — where the layout pages one pane at a time —
stored sizes are ignored and no handles exist: the paging layout is not
resizable, settled as part of the brief.

The details pane also gains a content cap: its content lays out at most
**60rem** wide — the page measure inherited from askance and still used by
the Set and Settings pages — and centers horizontally when the pane is wider
than that. The existing special case where a terminal screen fills the
pane's height must keep working; whether the cap applies to the terminal is
the builder's call, but prose, sheets and diffs in the pane must respect it.

## Acceptance criteria

- [ ] Every divider drags in its layout, sizes survive a reload on the same
      device, minimums hold, and a double-click restores the defaults
- [ ] Narrow layouts show no handles and ignore stored sizes entirely
- [ ] Details-pane content never exceeds 60rem and sits centered when the
      pane is wider, with the full-height terminal case still working
