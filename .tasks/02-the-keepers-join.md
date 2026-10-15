# 02. The Keeper's Join

## What to build

The Keeper links its Annex with a join of its own kind (ADR-0024, *The
Keeper's Join*; CONTEXT.md, **Keeper's Join**), and both ends record the
relation. This task is both ends of it plus the operation that starts it.

**The linking operation, on the Keeper.** Given the Annex's data directory and
its peer address:

1. Read the Annex's `device.pem` out of that directory and take its
   fingerprint — the same SHA-256 the device takes of its own.
2. **Write `kept_by`** — this device's own Device Id and *current*
   fingerprint — into the Annex's `config.yaml` where it differs from what is
   there, keeping every other key. The Keeper owns that directory, and the
   Annex re-reads its config on every call, so this is what keeps the pin good
   across the Keeper's own certificate renewals. (Decided at planning:
   rewritten here, rather than the Annex following renewals off its members
   row.)
3. **Dial the join pinned to the Annex's certificate** — the one join that
   never accepts whatever certificate is there. Today's join dial uses the
   unpinned client that remembers what it met; this is a sibling of it on the
   pinned client every member dial already uses.
4. Record what the reply carries.

It is reached as a workbench-key route, `POST /api/ui/devices/annexes` with
`{ data_dir, address }`, and no UI — stage 03 calls the same method at every
boot. **Idempotent**: run again, it rewrites nothing unchanged, the join is
accepted again, and each side still has one member row for the other.

**The join, on the Annex.** On a device with `kept_by`, the join route takes a
join whose handshake certificate matches `kept_by`'s fingerprint *and* whose
body names `kept_by`'s Device Id as accepted outright: **no held request, no
Nudge of the joins, no push, no expiry timer**. It records the Keeper as a
member and answers **in the join's own reply** with what an Allow would have
dialled back — its own identity and its roster — so the Keeper records the
Annex and the roster from the response. One round trip: no pending row on the
Keeper and no dial back. The reply is a new shape beside `JoinHeld`; a Keeper
that gets a `JoinHeld` instead treats the link as failed and cancels it rather
than leaving a pending row. Any other join an Annex receives is held and asked
about exactly as today.

**The mark.** The relation is a field on `DeviceIdentity`: a device with
`kept_by` says *kept by ‹Keeper's Device Id›* about itself, in its Device
Reading, so it is on the identity endpoint, the join body and the join reply.
(Decided at planning: the Annex says it, rather than the Keeper asserting it —
every refresh of a member, the identity dial's included, is rebuilt from a
`DeviceIdentity`, so a mark that is part of what the device says is never
wiped by one.) The members table gains a column for it, added the way
`reachable` was (declared in the CREATE, `ALTER TABLE … ADD COLUMN` where
`pragma_table_info` lacks it); `Member` carries it, and every place that turns
a `Member` back into a `DeviceIdentity` carries it through. `LinkedDevice`
gains it, for task 03 to draw.

Re-ground first: the join is held in `peer/joining.rs`, the dial-back on Allow
is answered in `peer/exchange.rs`, the pinned and unpinned clients are in
`peer/dialling.rs`, Add is `Devices::add` in `device.rs`, and the members table
is `crates/store/src/members.rs`. The two-device suite to extend is
`crates/server/tests/joining.rs`.

## Acceptance criteria

- [ ] A Keeper's Join from A to a B whose `kept_by` names A settles in one
      round trip: afterwards A's members include B, marked kept by A, and B's
      include A — with no held request on B, no Nudge of joins, no push and no
      pending row on A.
- [ ] A join to B from a device not named in `kept_by`, or from A presenting
      another certificate, is held and asked about as before.
- [ ] A linking operation whose `device.pem` does not match the certificate
      the address answers with is refused, and nothing is recorded on either
      side.
- [ ] Run twice, the operation leaves one member row on each side and
      `kept_by` holding A's current fingerprint; run after A's fingerprint
      changes, it rewrites `kept_by` and the join is still accepted.
- [ ] An identity dial that refreshes B's member row on A leaves the mark in
      place.
