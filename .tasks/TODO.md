# The app updates this device

On a desktop install, this device's Devices row has **Update** whenever a newer
stable release exists. A press shows a confirm card, *Update wolf?*, naming the
version and the sessions running here and saying they will be stopped and
resumed; on Update the app downloads the release with electron-updater while
the row reads *updating…*, the row reads *restarting…* once the server goes
quiet, and the app relaunches on the new version, which the row then shows. A
version that does not come up shows a dialog, and a failure leaves a trouble
line on the row. An install that cannot update itself — the NixOS module, a dev
checkout, a bare `verkstead serve` — shows its version and a line saying how to
update by hand, with no button. The Update Notice's link lands on a README
*Updating* section that exists.

The decisions are ADR-0027's, *A device updates itself* and *The release*, with
one change settled at this stage's start (Set 1404): the pinned electron-builder
has no manifest signing, so the update metadata is published unsigned on 26.
The exchange is proven against a local feed by a real electron-updater run in
the release workflow's Linux leg; the run against a published GitHub release
comes at the release after the first one carrying this.

Roadmap stage: [04: The app updates this device](docs/roadmaps/cluster-updates/04-the-app-updates-this-device.md)

## Tasks

- [x] 01: The release publishes update metadata — [details](01-the-release-publishes-update-metadata.md)
- [ ] 02: What this install is — [details](02-what-this-install-is.md)
- [ ] 03: Update, end to end, against a local feed — [details](03-update-end-to-end-against-a-local-feed.md)
- [ ] 04: When it goes wrong — [details](04-when-it-goes-wrong.md)
- [ ] 05: The confirm card — [details](05-the-confirm-card.md)
- [ ] 06: The README's Updating section, and the Notice's button — [details](06-the-readmes-updating-section-and-the-notices-button.md)
