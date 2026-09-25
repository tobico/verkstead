# 06. The Mac

## Goal

`Verkstead-universal.dmg` on a Release is the Electron app, a regular Dock
app carrying the two Mac CLI builds joined with `lipo`. Closing the window
leaves the app in the Dock and a Dock click brings the window back; Cmd+Q
quits at once. The traffic lights sit to the left of the Wordmark; the
Desktop page holds the menu bar icon (on by default), View Logs and Launch on
Startup, and nothing else; Launch on Startup is the login-item registration,
taking over the tray app's launch agent where there is one, and a login start
comes up hidden. The `desktop-macos` leg builds, mounts and runs it, and the
Rust launcher script and dmg script are gone.

## Decisions in force

- **A Mac is the platform's own** ([ADR-0020], Set 846 Q8): the menu-bar-only
  activation policy goes, the app is in the Dock, close hides and activate
  shows. The close radio and the quit warning are not drawn there (Q8a, with
  the human's note dropping the warning too).
- **The menu bar icon defaults on** (Q8b): one default everywhere. The menu is
  where View Logs lives while the icon is shown, and the Desktop page's own
  button is where it lives when it is not — the same rule stage 03 wrote for
  the tray.
- **Traffic lights left of the Wordmark** (Set 847 Q11), inset to the head's
  first row; the sidebar's head leaves room, as stage 04 wrote.
- **Launch on Startup through Electron's login-item API** (Set 846 Q9a): the
  app is always a bundle now, which is what the tray app's hand-written plist
  was working around. Still the state rather than a copy.
- **The tray app's plist is taken over, once.** It sits at
  `~/Library/LaunchAgents/net.tobico.Verkstead.plist` and starts
  `Verkstead.app/Contents/MacOS/Verkstead-launcher`, which this stage deletes;
  the login-item API knows nothing about it, so left alone it is a launch
  agent that fails at every login while the box reads off. The first launch
  after an upgrade reads the plist, carries whether it said `RunAtLoad` into
  the new registration, and removes it. Read once at launch rather than
  offered as a control: it is a registration this app made another way, not a
  second setting.
- **A universal dmg with the lipo'd CLI, unsigned** (Set 848 Q14, Q16a), the
  Gatekeeper steps in the adoption docs kept.
- **The leg proves the artifact**: mounted and run out of the mount, the same
  three assertions, and the binary inside answering `ask` by path.
- **Sessions on `sandbox-exec` are untouched**: what the app bundles is the
  same CLI, and the policy is the server's.

## Proposed tasks (provisional)

1. **Dock behaviour** — regular activation policy, close hides, activate
   shows, Cmd+Q quits without asking, the application menu kept. Accepts: a
   closed window returns from the Dock; Cmd+Q ends the sidecar.
2. **The page and the lights** — the Desktop page reduced on `darwin` to the
   menu bar icon, View Logs and Launch on Startup; the traffic lights
   positioned in the head row and the inset proven. Accepts: no control under
   the lights in any pane count; View Logs is reachable with the icon off.
3. **Login item** — the API arm enacted, hidden start when the icon is shown,
   and the one-shot take-over of the tray app's launch agent. Accepts: the box
   reads the registration; a login start shows no window while the icon is on;
   a home carrying the tray app's plist comes up registered through the API
   with the plist gone, and one carrying none is untouched.
4. **The dmg and the leg** — electron-builder's universal dmg with the two
   CLI artifacts lipo'd as an extra resource; the leg downloads both, packs,
   mounts and asserts. The Rust dmg and launcher scripts retired. Accepts: the
   leg is green; `Verkstead-launcher` exists nowhere.
5. **The words** — the Mac sections of adoption, releasing and development
   rewritten, development's build list dropping `tools/build-macos-dmg.sh` and
   its dmg paragraph describing the packed app — the `LSUIElement` that made it
   a menu-bar app with no Dock tile among what goes. Accepts: nothing names the
   launcher script or `verkstead desktop`.

## Re-verify at start

- Stage 05 landed: the builder configuration and the release plumbing exist.
- Which login-item mechanism Electron's pinned major uses on the runner's
  macOS, and what `openAsHidden` still means there.
- The CLI matrix still builds `verkstead-macos-x64` and `-arm64`.
- What `crates/desktop/src/startup/launchd.rs` writes today — the file's name
  and the keys `says_on` reads — which is what the take-over has to find, and
  which stage 08 deletes the only other reader of.
- The dmg leg's signature assertion, which was about a script and may have
  nothing left to say.

[ADR-0020]: ../../adr/0020-electron-desktop.md
