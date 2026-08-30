# Share fixes

Three gaps in the share feature, settled by grilling. The downloaded share
draws its cards and diffs without the padding the workbench gives them, because
the share document lacks the `main` column the styles hang those widths on. The
share's two panes cannot be resized, where the workbench's can. And a published
share's link still points at the raw gist — which a browser draws as source —
even though the share viewer page exists: nothing hosts it for the human, and
only pull-request comments compose links through it.

The fix lands in four slices: restore the share document's column, give its
panes the workbench's divider, host the viewer on this repository's GitHub
Pages, and make that hosted page the default so every link Verkstead hands out
displays the share directly.

## Tasks

- [x] 01: Give the share document the app's column — [details](01-share-column.md)
- [x] 02: A divider between the share's two panes — [details](02-share-divider.md)
- [x] 03: Host the share viewer on GitHub Pages — [details](03-viewer-hosting.md)
- [x] 04: Default the viewer URL and compose every link through it — [details](04-viewer-links.md)
