# 02. Profiles by uuid, bound per device

## Goal

An Agent Profile is one record on every device, and everything that names one
names its uuid. The account paths are a **binding** of one device, itself a
replicated row per device, so the Profile card carries a line per device —
bound with its path, or *runs from a copy*. At the switch, a device that has
members names each of its own unnamed Profiles after its hostname, and so does
a device the first time it links, so the four Claude rows — and the two of
every cluster formed after this release — read apart in every picker. A session on a device the Profile is
bound on runs exactly as today; one on an unbound device fetches the login from
a reachable bound device, the way it fetched from the home before, until 03
replaces that. Demonstrable with two Verksteads: one record bound on A, a
session on B under it starting with the login fetched from A, a Pairing on each
naming the uuid, and the old mirror rows gone. A second binding of one record
takes the press, which is 03's.

## Decisions in force

From [ADR-0028](../../adr/0028-records-are-the-clusters.md) and the pointers
on **Agent Profile** and **Account mirror** in `CONTEXT.md`; this stage
rewrites the **Agent Profile** term in full and the mirror parts of **Account
mirror**.

- **Every reference becomes the uuid**: the three pairing columns on a
  Conversation, the Repo's pairing memory, pending Steers, the models and
  homes tables, the account directory under the Data Directory, `ProfileAcross`
  and the peer `{profile}` paths, the `Login` Nudge and the viewer's id. A uuid
  column beside a kept local id was turned down.
- **The record is name, harness, models and the memory switch.** The account
  paths and whether a login file is held are **this device's binding**, a
  replicated row keyed by the Profile's uuid and the device, so every device
  can draw where the Profile is bound. A device without a binding runs from a
  copy under its Data Directory, named by the uuid.
- **Binding is a press, never automatic** — *Use this machine's own account*
  on the card with the path fields — because the login at `~/.claude` on
  another machine may be a different account. The press itself lands in 03
  with the copy; here every device's existing paths become its binding at the
  switch.
- **Unnamed Profiles are named after their device at the switch**, on a
  device that has members only, **and when a device first links**, once —
  otherwise every cluster formed after this release starts with two unnamed
  Profiles of one harness; a device in no cluster leaves them unnamed.
  Nothing merges — the human removes the extras by hand and may clear the
  keeper's name.
- **The uniqueness rules hold at the press** against the whole table: one
  unnamed per harness, no two named alike; a pair arriving by replication is
  kept and drawn, two alike.
- **Mirror rows are bridged, not dropped.** A mirror is `(home_device,
  home_id)`; the record the former home publishes carries the device it was
  minted on and the id it had there, which is the bridge. Until the home
  updates, the mirror holds a provisional uuid of this device's minting so its
  Pairings are already uuids; the bridge rewrites the provisional uuid to the
  real one. A member too old to publish is a trouble line naming both versions,
  its Profiles absent from the pickers meanwhile.
- **Removal is a tombstone**, and a tombstone arriving nulls every Pairing
  that named the uuid, as `forget_mirror` does today.
- **Brokenness stays per device**: the harness absent here, in the onboarding
  probe's words; not bound here and no bound device reachable, naming none.
- **The Transfer Preflight compares Profiles by uuid**, which is now the same
  uuid on both ends; `ProfileAcross` carries the uuid alone.

## Proposed tasks (provisional)

1. **The uuid everywhere.** Mint a uuid per local row; convert the seven tables,
   the account directory, `ProfileAcross`, the peer paths, `Nudge::Login` and
   `ProfileEntry.id`; mirrors get a provisional uuid; the account-mirror
   directory is renamed.
   - A Conversation's three Pairings and a Repo's pairing memory survive the
     rewrite and draw the same Profiles.
   - A transfer names the Profile by uuid and the far end resolves it.

2. **The record and the binding on the facility.** `profiles` splits into the
   replicated record and the per-device binding row, both on 01's facility;
   this device's paths become its binding at the switch; the pickers and the
   settings section read the merged table with no device marks.
   - A Profile made on A is in B's pickers with no binding on B.
   - Renaming on B redraws on A off the pulled record; removing on A tombstones
     and nulls B's Pairings.

3. **Bridging the mirrors and naming the unnamed.** The record carries where it
   was minted and the id it had; a device bridges its mirror by that pair,
   rewriting the provisional uuid; a device with members names its own unnamed
   Profiles after its hostname once, at the switch, and a device that first
   links does the same as it links.
   - Two devices updated in either order end with one record per Profile, each
     device's Pairings intact.
   - Four unnamed Claude Profiles read as four hostnames in a picker; a
     Verkstead in no cluster still shows one unnamed Claude.
   - Two Verksteads on this release, each with an unnamed Claude Profile,
     linked: each picker reads the two hostnames, not *Claude — Default*
     twice.

4. **The card's line per device and brokenness.** A line per device — bound
   with its path, or *runs from a copy*; brokenness answered per device.
   - The card on A says A is bound at its path and B runs from a copy.

5. **Launch on an unbound device.** The account fetch asks any reachable device
   whose binding holds a login, the account route addressed by uuid and
   answering for a bound Profile; the refusal names no bound device reachable.
   - A session on B under a Profile bound on A alone starts with A's login and
     writes it back to A; with A off it is refused naming the gap.

## Re-verify at start

- Assumes 01 landed: the facility, its peer route and puller, Authors on it.
- Assumes `profiles` is still `crates/store/src/profiles.rs` with
  `home_device`/`home_id`/`home_login`, `profile_models` and `profile_homes`,
  and the seven referencing tables are as the map in this roadmap's grilling
  found them (conversations' three columns, `repo_pairings`, `pending_steers`,
  the two child tables, the account directory under `accounts/<id>`).
- Assumes `forget_mirror`/`forget_pairings` are still how a removed Profile
  leaves its Pairings, and `arrivals.rs` still nulls the three columns.
- Assumes `ProfileAcross { device, id }` is still how a transfer names one and
  `here()` in `peer/transfers.rs` resolves it.
- Assumes the viewer's `ProfileList.tsx` still draws the device badge and the
  pickers list by harness and name.
- Assumes Windows' same-volume rule for the account and the Data Directory still
  holds and a copy under the Data Directory still satisfies it by construction.
