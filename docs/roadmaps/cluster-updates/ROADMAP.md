# Cluster updates roadmap

Every device in a cluster shows the release it runs, a failed call to a member
names the version gap, and the desktop app on each device updates itself with
electron-updater when the human presses Update on its row — one device, or
all of them from one page, members first and the device drawing the page
last. What forced it was winvm, a release behind wolf, unable to use wolf's
Agent Profiles with nothing on either screen saying why. The decisions and
their why are in [ADR-0027](../../adr/0027-cluster-updates.md);
[CONTEXT.md](../../../CONTEXT.md) carries the terms, **Devices** and **Update
Notice** amended by it.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

Stages 01, 02 and 03 stand on nothing and may land in any order: 01 is the
server and the viewer, 02 and 03 are the release workflow on one platform
each, and the human split the release work in two so that the Mac signing,
which waits on an Apple enrolment and secrets only they hold, holds nothing
else up. 04 is the app updating itself and stands on all three: on the
version line 01 draws, which is where its Update goes, and on 02 and 03
because electron-updater refuses an ad-hoc signed Mac update and does not
update an msi, so a 04 without them would offer Update on two of the four
devices and fail. The update metadata the release has to publish is 04's
own, because on Linux it is all that stands between the AppImage and an
update, and 04 is proven there. 05 stands on 04, there being nothing to tell a
member to do until a device can update itself.

## Stages

- [ ] 01: Versions across the cluster — [brief](01-versions-across-the-cluster.md) — no dependencies
- [ ] 02: A signed Mac release — [brief](02-a-signed-mac-release.md) — no dependencies
- [x] 03: The Windows installer — [brief](03-the-windows-installer.md) — no dependencies
- [ ] 04: The app updates this device — [brief](04-the-app-updates-this-device.md) — after 01, 02, 03
- [ ] 05: Updating members — [brief](05-updating-members.md) — after 04
