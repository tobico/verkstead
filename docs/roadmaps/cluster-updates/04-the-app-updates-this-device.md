# 04. The app updates this device

## Goal

On a desktop install, this device's Devices row has **Update** whenever a
newer stable release exists. A press shows a confirm card, *Update wolf?*,
whose body names the version and the sessions running here and says they will
be stopped and resumed; on Update the app downloads the release with
electron-updater while the row reads *updating…*, the row reads
*restarting…* once the server goes quiet, and the app relaunches on the new
version and the row shows that version. A version that
does not come up shows the app's dialog as today and leaves a trouble line on
the row. An install that cannot update itself — the NixOS module, a dev
checkout, a bare `verkstead serve` — shows its version and a line saying how
to update by hand, with no button. The Update Notice's link lands on a README
*Updating* section that exists. Demonstrable on Linux with the AppImage against
a published release, and in the suite against a local update feed.

## Decisions in force

- **electron-updater, the real one**
  ([ADR-0027](../../adr/0027-cluster-updates.md), *A device updates
  itself*). Chosen twice over a fetch-and-verify of Verkstead's own; its
  conditions are stages 02 and 03, and the update metadata is this stage's.
- **The server tells the app over the sidecar's stdout**, which the app
  already reads line by line and files into its log; no new channel. **The
  app reports back over the viewer's own API with the Workbench Key** it
  already reads out of the Data Directory, so the row's state is the
  server's to draw and a phone sees it too.
- **The confirm card names the sessions** and Update goes ahead; a restart
  stops them and startup re-drives each driven Conversation through the
  harness's own resume, as after any restart. Refusing while a session runs,
  as the Annex's Update does, was not taken.
- **The words are the deferred wording Set's answers** (1381). The card's
  question is *Update <device>?* — *Update crow?* — with the version in its
  body beside the sessions. The row reads *updating…*, as the Annex row's
  state word does, then *restarting…* once the device goes quiet. On this
  device's own row the server that draws the state is the one going away, so
  *restarting…* is the viewer's: a row reading *updating…* whose server stops
  answering reads *restarting…* until the page reconnects.
- **No rollback.** The dialog, the trouble line, and a higher release.
- **Stable releases only**, as the Update Notice counts them.
- **Signed update metadata.** `latest*.yml` is published beside the assets
  and signed with an Ed25519 key kept as a release secret; whether the
  pinned electron-builder has manifest signing at all is this stage's first
  check, and `updateManifest: false` is the fallback the human accepted only
  if the pin lacks it. The metadata is emitted with a `publish` provider
  configured and publishing itself left to the workflow's `gh release
  create`.
- **Installs that cannot update show the command.** The server detects
  what it was started from — an AppImage mount, a Mac bundle, the Windows
  install directory, a nix store path, a `target/` build, anything else — and
  the row says which and what to run. Nothing is attempted on them.
- **The Update Notice stays**, its banner now carrying the button stage 05
  gives it; here it carries this device's Update and links to a README
  section that exists.

## Proposed tasks (provisional)

1. **The release publishes update metadata** — the `publish` provider in the
   configuration, the Ed25519 key or the documented fallback, and the
   `latest*.yml` files uploaded with the assets.
   - A release on GitHub holds `latest-linux.yml`, `latest-mac.yml` and
     `latest.yml`, each naming an asset the release holds.
2. **What this install is** — the server reads its install kind off its own
   executable at startup, puts it on the Devices view for *this device*, and
   the row shows the by-hand line where it cannot update.
   - A NixOS module install reads *update this device with …* and has no
     button.
3. **The update exchange** — a route the row presses, the stdout line the
   server says, the app driving electron-updater on it, and the progress the
   app posts back with the Workbench Key, read as the row's state.
   - The row reads *updating…* from the press, then *restarting…* once the
     server stops answering, until the app relaunches.
   - A failed download leaves the row's trouble line with the updater's
     reason.
4. **The press** — Update on this device's row when a newer stable release
   exists, the confirm card naming the device, the version and the running
   sessions, and the README *Updating* section the Update Notice links to.
   - On wolf, the card reads *Update wolf?* with the version in its body.
   - With two sessions running, the card names both; Update proceeds.
5. **A feed for the suite** — electron-updater pointed at a local feed so the
   desktop tests can prove the exchange without GitHub.

## Re-verify at start

- Assumes stages 01, 02 and 03 landed: the version line on the row, a
  Developer ID Mac release with a zip, and the NSIS installer.
- Assumes `desktop/electron-builder.yml` still has `publish: null` with the
  comment that nothing updates itself, that `desktop/src/sidecar.ts` reads
  the child's stdout and stderr line by line, and that `desktop/src/key.ts`
  reads `workbench.key` afresh from the Data Directory.
- Assumes the app quits when the sidecar exits (`desktop/src/main.ts`), so
  electron-updater's relaunch has to be ordered before the server is
  stopped, and that `desktop/src/health.ts` waits 30 s for the server and
  shows a dialog on failure.
- Assumes the server already reads its own executable at startup in
  `crates/server/src/sandbox.rs` (`Executable::of_the_server`, with the nix
  wrapper unwrapped) and the sidecar's environment keeps `APPDIR`
  (`desktop/src/unmounted.ts`), which together say what the install is.
- Assumes the Update Notice is `crates/server/src/updates.rs` and
  `web/src/update/UpdateNotice.tsx`, mounted on the settings page and
  linking to `README.md#updating`, which does not exist.
- Assumes the Annex's Update (`crates/server/src/annex/booting.rs`,
  `web/src/settings/Components.tsx`) is the model for a row with a state
  word, a button, and a trouble line, and for polling while it reads
  *updating*.
- Assumes the running-sessions count for the card can be read off this
  device's own Conversations the way the status button reads a session's
  state.
