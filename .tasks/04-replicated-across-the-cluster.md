# 04. Replicated across the cluster

## What to build

Profiles and their bindings replicate on stage 01's facility. Each kind is
declared on the server's replicating side the way Authors are:

- the arm in the records route
- the spawn in the puller
- a moved Nudge (the existing `Nudge::Profiles` serves)
- a failed-read word, so a member too old to serve them has its Devices row
  name both releases, and its Profiles are absent here until it updates

A merge that moved anything re-holds whatever reads Profiles in memory and
redraws the pages.

**The mirrors retire**, the way Authors' did:

- **Each mirror row is written down as a record** under the uuid task 02
  bridged it to, stamped at the epoch and written by its home device. The
  home's real record wins the first pull. The mirror's paths are the home's,
  so they become no binding here.
- **A mirror still provisional** belongs to a member too old to publish. It is
  kept so its Pairings hold, **not served** on the records route, and **absent
  from the pickers and the settings section**. When that member's records
  arrive carrying the minted pair, task 02's store function rewrites it to the
  real uuid.
- **These go**: the mirror refresher with its reads of `/api/ui/profiles`, the
  relayed edit and remove hop to the home device, the home columns, and the
  *at home on* wording. An edit or a removal made on any device is a local
  write that replicates.
- **The device mark goes** from every Profile row: the card's OS badge and
  device name, and the picker row's ` · <device>`. There is no home to mark.
  Two alike that arrived by replication are drawn two alike.

**A removed record arriving** nulls every Pairing that named its uuid on this
device, as removing one here does.

**The launch under a Profile not bound here is refused for now**, because the
mirror's home fetch goes with the mirror. Task 05 restores it from a bound
device. The human accepted this gap inside the branch (Set 1476).

## Acceptance criteria

- [ ] A Profile made on A is in B's pickers and settings section, with no
      binding on B and no device mark.
- [ ] Renaming it on B redraws it on A off the pulled record.
- [ ] Removing it on A stamps it removed everywhere, and B's Conversations
      paired with it read as nothing picked.
- [ ] Two Verksteads linked before this release end with one record per
      Profile and their Pairings intact, and no mirror row left.
- [ ] A member too old to serve the route is its row's trouble line naming
      both versions, and its Profiles are absent from the pickers.
