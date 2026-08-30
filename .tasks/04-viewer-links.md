# 04. Default the viewer URL and compose every link through it

## What to build

Two changes that together make every link Verkstead hands out display the share
directly, with no setup.

**The default.** A blank `share_viewer_url` setting currently means "no viewer:
hand out raw gist links". Make it mean the canonical hosted viewer instead —
`https://tobico.github.io/verkstead/share-viewer.html`, the page task 03 put
up. The setting still overrides: a human who hosts their own viewer keeps
exactly the behaviour they have today. The settings section's wording changes
to match: a blank field uses Verkstead's hosted viewer, and filling it in
points links at your own.

**The links.** Only pull-request comments compose links through the viewer
today. The publish toast in the workbench and the conversation's share row hand
out the stored raw gist URL. Compose those through the viewer too — at display
time, from the stored gist URL plus the configured (or now default) viewer, the
way the existing `link` composition in the sharing module already does for
comments. Composing at display time means rows of shares published before this
change upgrade on their own, and a viewer URL changed later retargets every
link without republishing. The store keeps recording the gist URL untouched.

## Acceptance criteria

- [ ] With nothing configured, publishing a share yields a toast and a share
      row whose links read `…share-viewer.html#<gist-id>` and display the share
      directly; a configured `share_viewer_url` wins over the default.
- [ ] The share row of a Conversation published before this change also links
      through the viewer, without republishing.
- [ ] Pull-request comments behave as before, now defaulting to the hosted
      viewer when nothing is configured.
- [ ] The settings section says what a blank field means now; tests cover the
      default, the override, and the display-time composition.
