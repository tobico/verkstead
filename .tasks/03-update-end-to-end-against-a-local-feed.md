# 03. Update, end to end, against a local feed

## What to build

A plain **Update** on this device's row (no confirm card yet — task 05), shown
when the install kind from task 02 can update itself and a newer stable release
exists, that takes the device onto that release.

**The exchange** (ADR-0027, *A device updates itself*):

1. The press is a route under `/api/ui/` on this device's server. The server
   marks itself updating and **says so on its stdout**, one line in a fixed,
   greppable shape the app recognises (the sidecar's stdout already reaches the
   app line by line — `desktop/src/sidecar.ts`, filed into the log by
   `heard`). No new channel. A server not started by the app (`--desktop`
   absent) refuses the route by name.
2. The app, on that line, drives electron-updater: check, download, then
   `quitAndInstall`.
3. **The app reports back over the viewer's own API** with the Workbench Key it
   reads afresh from the Data Directory (`desktop/src/key.ts`): progress and,
   in task 04, failure. The row's state is the server's to draw, so a phone
   sees it too.
4. The row reads *updating…* from the press, polled the way the Annex row is
   while it reads *updating* (`web/src/settings/Components.tsx`,
   `crates/server/src/annex/booting.rs`). **On this device's own row the server
   drawing the state is the one going away**, so *restarting…* is the
   viewer's: a row reading *updating…* whose server stops answering reads
   *restarting…* until the page reconnects, then shows the new version.

**The relaunch ordering**, which is the trap here: the app quits when the
sidecar exits (`sidecar.gone` → `app.quit` in `desktop/src/main.ts`), and
`will-quit` stops the sidecar with a 5 s grace. electron-updater's
`quitAndInstall` must be what starts the quit, so the sidecar's going is
expected rather than news (`leaving`). And the relaunched app must not meet
the old one: the single-instance lock (a second instance exits) and the fixed
port 8422 (the probe shows the *foreign listener* dialog). Make the new launch
wait out the old one rather than refuse.

**The proof** (Set 1404, Q4): a script, runnable locally and run in
`.github/workflows/release.yml`'s Linux desktop leg (which already packs and
launches the AppImage under xvfb), packs the app at two versions, serves the
higher one's `latest-linux.yml` and AppImage from a local `generic` feed,
starts the lower one pointed at that feed, presses Update through the API, and
checks the relaunched app's server reports the higher version. The feed is
chosen by an override only a test sets, never by a packed app on its own.
Vitest covers the exchange's logic with electron-updater faked behind an
interface. See the memos on running the Electron app and a packed AppImage
here before trying it locally.

## Acceptance criteria

- [ ] From a press, the row reads *updating…*, then *restarting…* while the server is down, then the new version once the app has relaunched.
- [ ] The relaunch neither exits on the single-instance lock nor shows the foreign-listener dialog.
- [ ] The local-feed script passes in the release workflow's Linux leg and locally; the vitest suite covers the exchange with a faked updater.
