# 07. Naming the unnamed

## What to build

Nothing merges two Profiles: two unnamed Claude Profiles are two records until
the human removes one. So a device names its own unnamed Profiles after
itself, **once**, at two moments:

- **At the switch, on a device that has members.** Every device of an existing
  cluster brings its own unnamed Claude. Named, the four rows of a four-device
  cluster read as four devices instead of *Claude — Default* four times.
- **When a device first links**, whichever side of the link it is on. Without
  this, every cluster formed after this release starts with two unnamed
  Profiles of one harness.

A device in no cluster leaves its unnamed Profiles unnamed. So does a device
that has been named once already: the human may clear the keeper's name
afterwards and it stays cleared. *Own* means the records whose binding is this
device's from the switch, never one that arrived by replication.

- **The name is the device's name as the Devices pane draws it**: the hostname
  unless the human renamed the device (Set 1476).
- **Each unnamed Profile of the device takes that one name**, whatever its
  harness. The name rule is per harness since task 03, so a device's unnamed
  Claude and unnamed Codex may share it, the harness's mark telling them apart.
- **A clash with a name this device's table already holds for that harness**
  (two devices with one hostname, or a Profile already called that) is kept
  and drawn two alike, as a pair arriving by replication is. The naming is not
  a press, and the rule is held at the press.

The write is an ordinary record write, so it replicates.

Where a device first links: the joining side settles in the peer exchange, and
the allowing side in Allow. Annex keeping goes through the same membership
write. Find the one place a device goes from no members to one.

## Acceptance criteria

- [ ] Four devices each holding an unnamed Claude, updated, read as four
      device names in every picker.
- [ ] A Verkstead in no cluster still shows one unnamed Claude after the
      update.
- [ ] Two Verksteads on this release, each with an unnamed Claude Profile,
      linked: each picker reads the two device names, not *Claude — Default*
      twice.
- [ ] A name cleared by the human after the naming stays cleared across a
      restart and a later link.
