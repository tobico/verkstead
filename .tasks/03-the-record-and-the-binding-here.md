# 03. The record and the binding, here

## What to build

This device's own Profiles move onto stage 01's **Replicated record** facility,
split in two. This is the way stage 01 split Authors: records on one device
first, replication second (task 04).

- **The Profile record**: name, harness, models, the memory switch, and where
  it was minted and the id it had there (the bridge pair from tasks 01 and 02).
  It has a uuid, an `updated_at`, a deletion stamp and its writer, like every
  record on the facility.
- **The binding**: one per Profile per device, holding that device's account
  paths in the harness's shape and whether a login file is held there. The
  facility keys a record by `(kind, uuid)` alone, so a binding's uuid is the
  **composite `<profile uuid>/<device id>`**, with the facility unchanged (Set
  1476). Only the device the binding names ever writes it. A binding of a
  removed Profile goes with it.

**At the switch** (a take-up, once, as Authors' `take_up` did), each of this
device's own Profiles becomes a record under the uuid task 01 gave it, and its
paths become **this device's binding**. The take-up runs on every device, so
every device's existing paths become its binding. Binding a device by a press
is stage 03's.

**Everything that read the `profiles` table reads the record and this
device's binding**: the Profiles section of the settings page, the form, every
pairing picker, the launch, the brokenness reading, the login, and the
references task 01 converted. A Profile's references are uuid columns with no
foreign key behind them, since a record is nobody's to cascade from.

**Saving and removing are local writes**, stamped by the facility's rule:

- **The uniqueness rules are checked at the press against the whole table**:
  at most one unnamed Profile per harness, and no two Profiles of one harness
  named alike. The name rule is now *per harness* (Set 1476, amended into
  ADR-0028). A Claude and a Codex Profile may share a name, the harness's mark
  telling them apart. The refusals keep their words.
- **Removal is a deletion stamp** and nulls every Pairing that named the uuid,
  the way removing a Profile does today: both halves of each role, the Repo's
  pairing memory and a pending steer's picker. A recorded steer keeps naming
  the uuid and draws the name the removed record carries.

**The mirrors are still held** as they were, in what is left of the table or
beside the records, and drawn and picked as before. Task 04 retires them.

## Acceptance criteria

- [ ] After the switch, every Profile this device had reads, edits, picks and
      launches as before, its paths now this device's binding.
- [ ] Saving a second unnamed Claude, or a second Claude named alike, is
      refused in today's words. A Codex named like a Claude is saved.
- [ ] Removing a Profile stamps its record removed and leaves the
      Conversations and Repos that chose it with nothing picked.
- [ ] Mirrors of a member's Profiles are still listed and picked.
