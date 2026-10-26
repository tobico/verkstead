# Every device shows its version, and a desktop install updates itself on a press

A Verkstead in a cluster tells its members which release it runs, every
Devices row shows that version, and the desktop app on each device updates
itself with electron-updater when the human presses Update on its row — on
one device, or on all of them from one page.

What forced it: winvm, a Windows VM on an earlier release, could not use the
Agent Profiles wolf held on the latest one, and nothing on either screen said
why. A member's version crossed no wire, the identity record carrying a name,
an OS and addresses and nothing else; the Update Notice told the device
drawing it that a release existed and linked to instructions the README does
not hold; and nothing installed anything anywhere. Decided in the grilling of
2026-10-25 (Sets 1376 to 1386). The roadmap that builds it is
[`docs/roadmaps/cluster-updates/`](../roadmaps/cluster-updates/ROADMAP.md).

## Seeing versions

**The identity record carries the version.** The **Device Reading** every
device answers gains the release it runs, read off the binary the way the
name is read off the machine, and a member that sends none is a Verkstead
from before this. It is re-read whenever a member's Nudge stream is taken
up, which happens at every restart of either end — and a version changes
only across a restart — so what a row shows is fresh by construction, with
no poll for it. A route of its own, and a field in the discovery record,
were considered and not taken: both would say the same thing a second way.

**Every Devices row shows it, with two marks.** The version under the name,
as the Annex's Components row shows its own; drawn in a colour of its own,
with no word, when it differs from the device drawing the page; and a tag
when a newer stable release exists than the one that row runs. The version
alone, and the release tag alone, were offered and not taken: the differing
mark is what the incident wanted. The differing version as a bare coloured
number is the deferred wording Set's answer (1381); the newer-release tag
reads *update available*, set where *unreachable* stands and as that word is
— settled at the stage's review (1398).

**A failed call to a member names the gap.** Where a call to a member fails
and the two versions differ, the trouble line it leaves gains a sentence
naming both versions, such as *winvm is on 0.2.0 and this device is on
0.2.2.* Mirroring's lines today say only that a Profile could not be read.
The Member Gate's `Forbidden` already distinguishes a Verkstead too old to
have a route from one that will not have a caller; this is the same thought
on the asking side.

**No refusal.** Two Verksteads a release apart go on talking, on the
tolerate-it rules the peer code already follows; skew is shown, not
enforced.

## A device updates itself

**Each device updates itself when told.** The device drawing the page tells
a member over a peer call and the member does the rest with its own install,
fetching from GitHub Releases as the Update Notice's check already does. The
alternative, the pressing device downloading once and writing files onto
each member over the peer link, was rejected: it would have to know every
member's install layout, and it would make one device a writer of another's
program files.

**The desktop app does the whole of it, with electron-updater.** The human
chose electron-updater over a fetch-and-verify of Verkstead's own, with the
server exiting and the app only swapping, and chose it again once its
conditions were known (Set 1384), on the ground that each could be met. The
server tells the app to update over the sidecar's stdout, which the app
already reads line by line; the app reports progress back over the viewer's
own API with the Workbench Key it already holds, so the row reads
*updating…*; the app downloads, verifies and relaunches as electron-updater
does on each platform. The real electron-updater is the point: its
conditions are the release's, below.

**A confirm card names the sessions that will be stopped.** A restart stops
every session on that device, and startup already re-drives each driven
Conversation through the harness's own resume, as after any restart. So the
card says they will be stopped and resumed, and Update goes ahead. Refusing
while a session runs, as the Annex's Update does, and waiting until the
device is idle, were both offered and not taken.

**No rollback.** electron-updater has no such step on any platform, and
Squirrel.Mac and NSIS each replace the install themselves; a rollback would
be built around the tool, against it. A version that does not come up shows
the app's dialog as today, the Devices row carries the trouble line, and the
fix is a release with a higher version. Keeping the previous file beside the
new one was the first pick and was given up for this once electron-updater
was settled.

**Stable releases only**, as the Update Notice counts them: a hyphenated
version is marked a pre-release and stays off `releases/latest`.

## The release

electron-updater's conditions, checked against electron-builder's own
documentation rather than remembered, each a thing the human agreed to:

- **macOS: a Developer ID signature and notarization**, with a `zip` target
  beside the dmg, since Squirrel.Mac refuses an ad-hoc signed update and
  `latest-mac.yml` is written off the zip. The human enrols in the Apple
  Developer Program as an individual and puts the certificate and a
  notarization credential into the release workflow's secrets. This also
  ends the *Open Anyway* step every Mac install has needed. Leaving the Mac
  to update by hand was offered and not taken.
- **Windows: NSIS in place of the msi.** electron-updater updates NSIS
  installs and not msi ones. The msi was kept over NSIS in ADR-0020; asked
  again with that known, the human let it go, with no reason for the msi to
  stay. A per-user install with no admin, and the CLI on the user PATH
  through an include script, are what NSIS has to keep. An installed msi is
  not upgraded by an NSIS installer, so the first NSIS release is installed
  over it by hand once, and the msi's upgrade test in the release workflow
  goes with the msi. Shipping both installers was offered and not taken.
- **Linux: the AppImage as it is**, which electron-updater replaces in
  place.
- **Signed update metadata.** electron-builder writes `latest*.yml` beside
  the assets, and its current documentation has it refuse to emit them
  without an Ed25519 signing key unless `updateManifest` is turned off. The
  human chose the key, generated once and kept as a release secret, knowing
  a lost key means one by-hand reinstall everywhere. Turning the manifest
  signing off and trusting TLS alone was offered and not taken. Whether the
  pinned electron-builder has this at all is to be verified when that stage
  starts.

  **Verified at stage 04's start (2026-10-26), and it has not.** The pinned
  electron-builder 26.15.3 and the latest stable 26.18.0 have no manifest
  signing at all; the Ed25519 `updateManifest` exists only in
  27.0.0-alpha, verified by electron-updater 7.0.0-alpha. Offered the alpha
  pair, the human took the fallback instead (Set 1404): stay on 26 and
  publish the metadata unsigned, each file carrying its sha512 and fetched
  over TLS, with the Mac's Developer ID check still standing. On 26 there is
  no `updateManifest: false` to write — it is simply not there. Signing
  comes with a stable 27.
- **The README gains the Updating section** the Update Notice has linked to
  since the public-release roadmap, and `docs/releasing.md` follows the
  workflow.

## Across the cluster

**Update on every Devices row, and Update all on the Update Notice.** The
Annex's Components row — a version line, a state word, an Update button and
a trouble line — is the shape each Devices row takes. A pane of its own
listing every device was considered and not taken; the rows are already the
list.

**Members first, this device last.** The device drawing the page restarts
too, and its restart ends the page's view of the others, so Update all tells
every reachable member first and updates itself once each has come back on
its new version or left a trouble line. All at once, with the page
reconnecting, was offered and not taken.

**A member unreachable at the press is skipped**, with a line on its row
saying so; the human presses again when it is awake. Remembering the press
and telling the member when it next appears was the recommendation and was
not taken.

**Click only.** No automatic updates for now; the design leaves room for a
setting later. A cluster-wide setting that updates an idle device when a
stable release appears was offered and not taken.

**A notification only on trouble**: a device that could not be updated. The
rows show each device coming back on its new version, and a notification per
device was offered and not taken.

## Out of scope, by decision

**Installs that cannot update themselves show the command.** A NixOS module
install runs a store path as an unprivileged user; a dev checkout runs
`target/debug`; a bare `verkstead serve` is supervised by nothing. Each shows
its version and a line saying how to update by hand, and is never attempted.
A module option that has the unit run a binary Verkstead keeps in its state
directory, a sudoers rule for `nixos-rebuild`, and a Rust fetcher with a
re-exec for bare binaries against a SHA256SUMS asset, were each offered and
not taken — the last given up once electron-updater owned the desktop half
and no device of the human's ran one. No SHA256SUMS is published.

## Consequences

- The **Update Notice** is no longer *informs only*: on a desktop install its
  banner carries Update all, and the glossary entry says so. The daily check
  behind it is unchanged.
- The Windows package changes shape once, and its install on an existing
  machine is a by-hand step that one release's notes have to say.
- The Mac release depends on secrets only the human holds; until they exist
  that stage cannot be proven, and the Linux and Windows halves can.
- A member from before this records no version, and its row shows none,
  which is itself the sign to update it.

## Considered options

- **The pressing device pushes the bits to each member.** Rejected: every
  member's layout known to one device, and one device writing another's
  program files.
- **The server fetches and verifies, the app swaps and relaunches.** The
  recommendation, twice; rejected by the human for electron-updater.
- **Refuse members on a different version.** Rejected for showing skew; the
  peer code's tolerate-it rules stay.
- **Rollback to the previous file.** Picked, then given up with
  electron-updater.
- **Remember a press for a member that is asleep.** Rejected for a line on
  the row and a second press.
- **Automatic updates when idle.** Deferred.
- **NixOS self-update through a state-directory binary or sudoers.**
  Rejected for the by-hand line.
