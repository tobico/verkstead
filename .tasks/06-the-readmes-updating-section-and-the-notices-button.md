# 06. The README's Updating section, and the Notice's button

## What to build

- **The README's *Updating* section**, which the Update Notice has linked to
  (`https://github.com/tobico/verkstead#updating`, in
  `web/src/update/UpdateNotice.tsx`) since the public-release roadmap and
  which does not exist — the README today has no section for it. It says a
  desktop install updates itself from its Devices row, and gives each by-hand
  kind's command in the same words task 02's row lines use (NixOS module, dev
  checkout, a bare binary). The anchor must be exactly `#updating`.
- **The Update Notice's banner carries this device's Update** where this
  device can update itself, going through the same confirm card as the row
  (task 05). Stage 05 adds *Update all* to it later; this is the one-device
  button. Where it cannot, the banner keeps its link. The Notice stays mounted
  where it is, on the settings page.
- **`docs/releasing.md` follows the workflow**: the metadata files and
  blockmaps a Release now holds, that they are unsigned and why (ADR-0027,
  amended), and that the run against a published release happens at the
  release after the first one carrying electron-updater — the first can only
  be updated *from*, and rehearsal tags are pre-releases the updater skips
  (Set 1404, Q5).
- Check the `CONTEXT.md` entry for **Update Notice** still says what the
  banner does.

## Acceptance criteria

- [ ] The Notice's link lands on a README section that exists, naming the same by-hand commands the rows do.
- [ ] On a desktop install with a newer release, the Notice's banner has Update, which opens the same confirm card as the row.
- [ ] `docs/releasing.md` says what the update metadata is and when the published update is first run.
