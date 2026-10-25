# 01. Versions across the cluster

## Goal

Every row in the Devices section shows the release that device runs, under
its name, the way the Annex's Components row shows its own — this device's
row included. A row whose version differs from this device's carries a tag
saying so, and a row whose version is older than the latest stable release
carries a tag saying a newer release exists. A member from before this stage
shows no version, which is itself the sign to update it. Where a call to a
member fails and the two versions differ, the trouble line it leaves names
both versions in a sentence. Demonstrable with two devices a release apart:
the winvm incident, as it should have read.

## Decisions in force

- **The identity record carries the version**
  ([ADR-0027](../../adr/0027-cluster-updates.md), *Seeing versions*). The
  **Device Reading** gains the release off the binary; a member that sends
  none is an older Verkstead, and the field is optional on the wire for it.
  A route of its own and a field in the discovery record were not taken.
- **Fresh by construction, with no poll.** A version changes only across a
  restart, and a restart of either end reopens the member's Nudge stream; so
  the identity is re-read when that stream is taken up, and nothing polls for
  it. Ordinary members are not re-read on any timer today, and this stage
  does not add one.
- **Two tags, both.** *Differs from this device* and *a newer release
  exists*. The version alone, and the release tag alone, were offered and not
  taken. The newer-release verdict is the Update Notice's own daily check,
  which is stable releases only and in memory; the tag compares each row's
  version against the release that check found.
- **The gap is named, not enforced.** No refusal on skew; the peer code's
  tolerate-it rules stay. The sentence is the one place the incident becomes
  legible, so it belongs on every trouble line a member's call can leave,
  mirroring's first.
- **The words of the tags are the deferred wording Set's** (Set 1381, whose
  answers reach this stage's sessions), and the trouble line follows
  `docs/copy.md`'s refusal rule: what is true, then the one action.

## Proposed tasks (provisional)

1. **The version on the Device Reading and the Member row** — the release in
   the identity record and stored on the member, read back into the Devices
   view for every row and for *this device*.
   - A member one release behind shows that release; one from before this
     stage shows none.
   - The view's own device carries its version.
2. **Re-read on take-up** — a member's identity read again whenever its Nudge
   stream is taken up, so a member that restarted on a new release shows it
   without a press.
   - Restart a member on a new binary; its row changes without reloading.
3. **The two tags** — *differs* against this device's version and *newer
   release* against the Update Notice's verdict, drawn as the *unreachable*
   tag is.
   - With the check off or unanswered, only the *differs* tag can appear.
4. **The gap on the trouble line** — a member's call that fails while the
   versions differ leaves a line naming both.
   - winvm's Profile failure reads *winvm is on 0.2.0 and this device is on
     0.2.2.* beside what it already said.

## Re-verify at start

- Assumes the wire record is `DeviceIdentity` in `crates/render/src/device.rs`
  with `kept_by` as the model of an optional field, and the stored row is
  `Member` in `crates/store/src/members.rs`, whose columns are added with
  `ALTER TABLE` and no migration machinery.
- Assumes a member's identity is written by `record_member` and read only on
  a dial, a join, an exchange and an announcement, and that the Nudge stream
  to each member is held by `crates/server/src/relaying/freshness.rs`
  (`stay_fresh`), which is where the take-up is.
- Assumes the Update Notice's verdict is `crates/server/src/updates.rs`'s
  in-memory `Current | Available{version}` and does not expose the running
  version, and that `/api/ui/update` is its one reader.
- Assumes the rows are drawn in `web/src/settings/Remote.tsx`'s Devices
  section from `useDevices()` in `web/src/devices.ts`, and the Annex row in
  `web/src/settings/Components.tsx` is the model for a version line.
- Assumes mirroring's trouble lines are the generic ones in
  `crates/server/src/mirroring.rs` and `mirroring/authors.rs`, and merging
  and relaying leave lines of their own that want the same sentence.
