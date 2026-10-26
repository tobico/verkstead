# 01. The release publishes update metadata

## What to build

A Release carries the files electron-updater reads, and the packed app knows
where to read them from — nothing yet reads them, but every packed app from
here on can.

- `desktop/electron-builder.yml` trades `publish: null` (and the comment saying
  nothing updates itself) for a `github` provider naming the repository, so
  electron-builder writes `latest-linux.yml`, `latest-mac.yml` and `latest.yml`
  beside each leg's artifacts and an `app-update.yml` into the packed app.
  **Publishing stays the workflow's**: the pack runs with publishing off
  (`--publish never` or the equivalent), and `.github/workflows/release.yml`'s
  `gh release create` uploads the metadata and each artifact's `.blockmap`
  beside the assets it already uploads. Each leg's metadata must name the
  asset name the release actually holds (`Verkstead-x86_64.AppImage`,
  `Verkstead-universal.zip`, `Verkstead-x86_64.exe`).
- **Unsigned metadata, by decision** (ADR-0027 amended, Set 1404): the pinned
  electron-builder 26.15.3 has no manifest signing — only 27.0.0-alpha does —
  so stay on 26; each file carries its sha512 and is fetched over TLS. There
  is no `updateManifest` key on 26 to write. Say so in the configuration's
  comment.
- **Stable releases only**: pre-releases stay off (`allowPrerelease` false,
  the default), matching the Update Notice's `releases/latest`.
- **electron-updater becomes the app's first runtime dependency**, pinned to a
  6.x release (6.8.11 is the latest stable at planning). `package.json` has no
  `dependencies` today and `electron-builder.yml` says "this app has no
  dependencies at all" under `npmRebuild: false` — correct that comment, and
  check the pnpm layout packs the dependency into `app.asar` (read the packed
  asar back; electron-builder 26 handles pnpm, but prove it). The pnpm
  workspace denies install scripts; electron-updater has none.
- The release legs that already read packed bundles back gain a check that the
  metadata exists and names the uploaded file; the release job's checksum
  listing covers the new files as it covers the rest.

## Acceptance criteria

- [ ] A rehearsal release holds `latest-linux.yml`, `latest-mac.yml` and `latest.yml`, each naming an asset that release holds, with its sha512 matching.
- [ ] The packed app carries an `app-update.yml` naming the `github` provider and this repository, and `electron-updater` is inside the packed app.
- [ ] Nothing is published by electron-builder itself; `gh release create` stays the only writer of the Release.
