# 01. Remove the share viewer configuration

## What to build

The share viewer stops being configurable: every Published Share link composes
through the hosted copy, always. Remove the setting end to end — the config
file key (`share_viewer_url`), the fields it rides in the settings API both
ways, the settings page's share viewer section together with its opening word
and route, the viewer download endpoint, and the compiled-in viewer copy the
download served. Link composition loses its override parameter and reads the
hosted address alone.

Self-hosting goes with it deliberately: without the override a self-hosted copy
can never be pointed at, so the download offer is dead weight. The page itself
survives — the repository's `share-viewer.html` and the Pages workflow that
publishes it *are* the hosted copy — and the cross-check that holds the hosted
address consistent must survive the settings-page copy of it going: tie the
server's constant to the Pages workflow directly.

## Acceptance criteria

- [ ] The settings page carries no share viewer section and no download offer,
      and the section's details route is gone.
- [ ] Published Share links open through the hosted viewer exactly as an
      unconfigured Verkstead's did before the change.
- [ ] A config file still carrying `share_viewer_url` is read without error,
      and the key is dropped by the next save.
- [ ] A test still ties the hosted address in the server to the Pages workflow
      that publishes the page.
