# 02. The mirrors bridged

## What to build

A mirror held from before the switch keeps its Pairings once its home device
updates. ADR-0028 says mirror rows are **bridged, not dropped**.

Since task 01, each of a device's own Profiles remembers **where it was minted
and the id it had there**: its Device Id and its old integer id. And each
mirror held from before the switch has a **provisional uuid**, with the home
device and the id it had at home still on it. The bridge matches the two
pairs:

- A member's Profile list (`/api/ui/profiles` as the refresher reads it)
  carries, for each of its own Profiles, where it was minted and the id it had
  there.
- When the refresher reads a member, a held mirror whose home device and home
  id match a Profile's minted pair, under a different uuid, is **rewritten to
  the member's uuid**. Every reference that named the provisional uuid follows
  in the same transaction: every reference task 01 converted (the Pairing
  columns, the Repo's pairing memory, the pending and recorded steers, the
  models and homes, and the account mirror directory under the Data Directory).
  It is a rewrite, so nothing is nulled.
- Write the rewrite as **one store function**. Task 04 calls it again for a
  mirror whose member was too old to answer here and updates after the move
  onto the facility.

**A member too old to publish** the minted pair (or whose list no longer
parses, its id still an integer) is a failed read. Its Devices row says so,
naming both releases, under ADR-0027's rule, the way stage 01's Authors route
already does. Its mirrors stay as they are, provisional, with their Pairings
kept, until it updates.

## Acceptance criteria

- [ ] Two Verksteads linked before this release, each with a Conversation
      paired with the other's Profile, end with one uuid per Profile on both
      ends and every Pairing intact. This holds whether A or B updates first.
- [ ] A mirror's account directory under the Data Directory follows the
      rewrite, so the next session under it fetches into the renamed one.
- [ ] A member still on the old release is a trouble line naming both
      versions, and the mirrors held of it keep their Pairings.
