# 05. Proven in the release

## What to build

The release workflow proves the Install Offer the way it proves everything
else about the packed app: on the uploaded file, with `--install` standing in
for the press nobody on a runner can make.

**The Linux leg** gains a step after the mount checks: a temporary home with a
`Downloads` of its own, the uploaded AppImage copied into it, and the app
started from there with `--install` and the Data Directory the leg already
says. Asserted off the file system and the log: `~/Applications/
Verkstead-x86_64.AppImage` is there and executable and the download is gone,
`~/.local/share/applications/net.tobico.Verkstead.desktop` names it, the icon
set is under `~/.local/share/icons/hicolor`, and the app's log says a launch
came up with `$APPIMAGE` under `~/Applications` and served. Bounded, as every
launch in the leg is.

**The Mac leg** installs out of the mounted image with `--install` in place of
the `ditto` into `/Applications` it does by hand today, before its shown launch
asserting the window and the menu bar icon — which otherwise meets the dialog
and hangs. Asserted: the relaunched app's executable path is under
`/Applications` and not translocated, the bundle is registered with
LaunchServices, and the steps that follow run on the installed app as they do
now.

A leg that cannot install fails the release rather than skipping. The Windows
leg is untouched. `docs/development.md` gains its line on the Install Offer
beside the Desktop section's account, and `docs/releasing.md` follows the
workflow where it describes these legs.

## Acceptance criteria

- [ ] The Linux leg installs from its Downloads into its home and asserts the file, the entry, the icons and the relaunch; the Mac leg installs out of the mount and asserts the untranslocated relaunch; both are green on a release build.
- [ ] A failed install fails the leg with a line saying what was not found.
- [ ] `docs/development.md` and `docs/releasing.md` say what the legs now do.
