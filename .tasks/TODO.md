# Picture previews in the diff

A file a browser can draw — an SVG, a PNG, any of the formats the Code pane
opens as a picture — is drawn as a picture in the diff rather than read as
code: Before and After side by side for an edit, one picture for a file added
or removed, in a question set's attached Diff and a commit's pane alike. The
bytes are stored nowhere new: every file in a patch names its exact bytes by
git's blob ids, and the viewer fetches a picture by that id out of the
repository the diff came from, as its fold scrolls near.

Settled in the grilling of 2026-10-23 and recorded in ADR-0026, with the
vocabulary in CONTEXT.md under **Diff** — both written ahead of the tasks, so
each task builds to words already there.

## Tasks

- [x] 01: A picture is served by its blob id — [details](01-a-picture-is-served-by-its-blob-id.md)
- [x] 02: The renderer marks a picture and the viewer draws it — [details](02-the-renderer-marks-a-picture-and-the-viewer-draws-it.md)
- [x] 03: A tap opens a picture at full size — [details](03-a-tap-opens-a-picture-at-full-size.md)
- [ ] 04: A share carries its pictures — [details](04-a-share-carries-its-pictures.md)
