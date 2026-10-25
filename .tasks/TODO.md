# Versions across the cluster

Every row in the Devices section shows the release that device runs, under its
name, the way the Annex's Components row shows its own — *this device*'s row
included. A version that differs from this device's is drawn in a colour of its
own with no word, and a row older than the latest stable release carries an
*update available* tag. A member from before this stage shows no version, which
is itself the sign to update it.

And a failed call to a member names the gap. What forced this was winvm, a
release behind wolf, unable to use wolf's Agent Profiles with nothing on either
screen saying why: mirroring's failures reach only the log. So a member's failed
background read is drawn on its own Devices row, and that line and every press
refused by a member gain *winvm is on 0.2.0 and this device is on 0.2.2. Update
winvm.* where the versions differ. Decisions in
[ADR-0027](../docs/adr/0027-cluster-updates.md), *Seeing versions*; answered in
Question Sets 1381 and 1394.

Roadmap stage: [01: Versions across the cluster](docs/roadmaps/cluster-updates/01-versions-across-the-cluster.md)

## Tasks

- [x] 01: The version on the Device Reading and the Member row — [details](01-the-version-on-every-row.md)
- [ ] 02: Re-read on take-up — [details](02-re-read-on-take-up.md)
- [ ] 03: The two marks — [details](03-the-two-marks.md)
- [ ] 04: A member's failed read on its row, naming the gap — [details](04-a-failed-read-on-the-row.md)
- [ ] 05: The gap on a refused press — [details](05-the-gap-on-a-refused-press.md)
