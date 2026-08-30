# 03. Host the share viewer on GitHub Pages

## What to build

The share viewer page (`crates/server/share-viewer.html`) turns a published
share — a secret gist — into a page that displays directly, by fetching the
gist in the reader's own browser and drawing it in a sandboxed frame. Today the
human must host that page themselves; nothing does it for them, so in practice
published links point at raw gist source.

Add a GitHub Actions workflow that publishes the viewer to this repository's
GitHub Pages, so there is one canonical, always-up copy:

- Triggered by pushes to `main` that touch the viewer file (plus a manual
  trigger for the first run and for recovery).
- Follows the conventions of the existing workflows: every action pinned to a
  released version, the runner pinned to an image, permissions the minimum
  Pages deployment needs (`pages: write`, `id-token: write` on the deploy job —
  nothing gets `contents: write`).
- The repository has no Pages site yet; the workflow enables it on first run
  (deploy-from-Actions source), rather than requiring a manual settings step.
- The artifact carries the viewer under its own name, so the page lands at
  `https://tobico.github.io/verkstead/share-viewer.html`. That exact URL is
  what task 04 bakes in as the default, so it must not drift.

## Acceptance criteria

- [ ] After the workflow runs on `main`, the viewer is up at
      `https://tobico.github.io/verkstead/share-viewer.html`.
- [ ] Opening that URL with a published share's gist id in the fragment
      (`…share-viewer.html#<gist-id>`) displays the share.
- [ ] A later change to the viewer file on `main` republishes it without any
      manual step.
