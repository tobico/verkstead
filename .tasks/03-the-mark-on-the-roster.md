# 03. The mark on the roster, and Unlink refused

## What to build

The membership is the Keeper's, and every device says so (ADR-0024, *The
Keeper's Join*; CONTEXT.md, **Annex**). Task 02 put the mark on
`DeviceIdentity`, the members table and `LinkedDevice`; this task carries it
across the cluster, draws it, and refuses Unlink on it.

- **The roster.** The exchange's roster and every announcement already send a
  `DeviceIdentity` per member, so the mark rides along. Make sure a third
  device records it — including from an announcement replayed after it was
  owed — and that the Keeper announces its newly linked Annex to the members
  it already has, the way an Allow announces a newcomer.
- **The Devices row.** On every device, an Annex's row draws its OS word and
  hostname as before and, beside them, *Annex of ‹Keeper's name›* — *Annex of
  this device* on the Keeper itself — and **no Unlink**. The Keeper's name is
  the one the cluster holds for that Device Id. Check the words against
  `docs/copy.md`.
- **Refused everywhere.** The workbench unlink route refuses a device marked
  as an Annex, naming it — on the Keeper and on every other member. An Annex
  refuses to unlink its Keeper, naming it. Workbench refusals are
  sentence-case sentences with a closing period, as the other `/api/ui`
  refusals are. A Keeper forgetting its own Annex is stage 04's, through a
  path that is not Unlink.
- **The Keeper's row on the Annex** draws without Unlink too, since the
  Annex's route refuses it.

Re-ground first: `LinkedDevice` is in `crates/render/src/device.rs`, the row
and its Unlink are `Row` in `web/src/settings/Remote.tsx`, the unlink route
ends in `Devices::unlink` in `crates/server/src/device.rs`, and the
three-server suite is `crates/server/tests/announcing.rs`.

## Acceptance criteria

- [ ] Three servers: A keeps B and is already linked to C; after the Keeper's
      Join, C lists B marked as A's Annex, and A lists it as this device's.
- [ ] Unlinking B through the workbench route is refused by name on A and on
      C; unlinking A on B is refused by name; nothing is forgotten on any of
      them.
- [ ] The Devices row draws the mark and no Unlink for an Annex and for the
      Keeper on its Annex, and an ordinary member's row is unchanged (web
      suite).
