# Ignored pull request comments

Two things stop Wrapping spinning up sessions about comments nobody wants
addressed. The Share to Pull Request comment — Verkstead's own, posted by the
configured token — gains an invisible marker and is dropped built-in, because an
author rule could never tell it from the human's own comments. And a
configurable ignore list lets the human silence classes of comments they never
want addressed (a misconfigured bot nagging about billing on every pull
request): regex rules, edited in the GitHub and git author settings pane,
applied everywhere Wrapping reads comments.

## Tasks

- [x] 01: The share comment stops triggering Wrapping — [details](01-share-comment-marker.md)
- [x] 02: Ignore rules live in the settings and round-trip the API — [details](02-ignore-rules-setting.md)
- [x] 03: Wrapping skips ignored comments everywhere it reads them — [details](03-wrapping-skips-ignored.md)
- [x] 04: The row editor in the GitHub and git author pane — [details](04-ignore-rules-editor.md)
