# 05. The viewer page and its setting

## What to build

The small static **viewer page** that turns a gist link into an in-browser
read, and the setting that tells Verkstead where the human published it.

A gist link alone shows source: GitHub renders gists as code, and the raw URL
is served `text/plain` with `nosniff`, so browsers refuse to render it. But
gist raw URLs answer cross-origin requests (`Access-Control-Allow-Origin: *`,
verified during grilling), so a single static HTML page can close the gap:

- It takes the gist id in the **URL fragment** (`viewer-url#<gist-id>`), so
  the id never reaches the viewer host's logs.
- In the recipient's browser it resolves the gist through the GitHub API /
  raw URL — straight from GitHub, through nobody else's server — using the
  raw content, not the API's truncated-at-1MB file body.
- It renders the fetched share in a **sandboxed iframe** (scripts allowed,
  same-origin not), so the share's own JS runs without owning the viewer's
  origin.

Verkstead ships this file and the human hosts it once, on a public GitHub
Pages site of their own — the settings page should offer the file and say
that much. A **share viewer URL** field in the settings records where it
lives; it is configuration, not a secret. Task 06 reads it when composing the
PR comment.

## Acceptance criteria

- [ ] Opening `viewer-url#<gist-id>` on a published share renders the full
      share in the browser with no download step, and the only hosts touched
      besides the viewer's own are GitHub's.
- [ ] A multi-MB share renders whole — nothing truncated at the API's 1MB
      file cap.
- [ ] The viewer file is obtainable from Verkstead with instructions, and the
      viewer URL setting round-trips through the settings page.
