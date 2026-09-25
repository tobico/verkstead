# 02. Linking

## Goal

Two Verksteads become one cluster, and a third joins through either. On A, the
Devices section of Remote access has **Add**: type B's address, and A's list
shows a pending row,
*Waiting for confirmation on B*, with A's fingerprint and Cancel. On B, every
open workbench raises a modal — A's name, OS, address and fingerprint, Allow
and Deny — and B's phones get a push. Allow exchanges certificates, B hands A
every member and announces A to each of them over its own link, with no further
press anywhere; both lists now read the same. Unlink drops a device from the cluster for everyone,
asked once. A member that stops answering stays on the list dimmed
*unreachable*. Demonstrable end to end with three servers.

## Decisions in force

- **A cluster is a membership, not a set of pairs**
  ([ADR-0020](../../adr/0020-cluster-mode.md), *A cluster is a membership*).
  One confirmation joins the newcomer to everyone, and **the introducer is what
  announces it** to each member over the link it already holds — no member
  confirms that announcement, because it arrives over a link that member has
  verified. A newcomer introducing itself was rejected: nothing on the wire
  would carry the vouching, so a member could not tell the newcomer from
  anybody else able to reach its peer port, and every member but the one that
  confirmed would be joinable without a press.
- **The join protocol** is the ADR's: A accepts B's certificate for the one
  call and posts id, name, OS, addresses and certificate; B holds the request
  ten minutes and asks; on Allow B dials A back, checks the certificate it
  meets is the one in the request, and hands over its own and every member's;
  A checks B's is the one it saw.
- **Fingerprints on both sides.** The modal shows A's; A's pending row shows
  the same, for comparison by eye. The human chose this over name, OS and
  address alone.
- **The modal is raised by a nudge in every open workbench of B and by a
  push**, held ten minutes, then expired; a deny or an expiry reads on A's
  pending row and is dismissed. A blocking dialog on A was rejected.
- **Every device advertises all its addresses on every exchange**, a peer
  trying them in order; the typed address is just the first known.
- **Unlink removes the device for everyone**, asked once as Remove on a Repo
  is; cutting only this device's own pair was rejected. Unreachable members
  stay listed, dimmed, Unlink still working.
- **Members are a store table**: id, name, OS, addresses, fingerprint, last
  seen. Stage 01's per-route membership check now has something to consult.
- **The join and the dial-back stand outside that check**, as stage 01's third
  decision says they must: a join arrives from a non-member, which is what a
  join is, and the dial-back answering it arrives before A has recorded B.
  Neither is un-authenticated for it — the join's certificate is pinned into
  the pending request, and the dial-back is matched against the certificate
  that request is holding — and both are refused once the ten minutes are up.
  The announcement in task 4 needs no exemption, being a member's own call.

## Proposed tasks (provisional)

1. **The member store and the gate** — the members table, and stage 01's
   per-route membership check reading it.
   - A call from a member's certificate reaches the gated peer API; one from a
     stranger is refused there.
   - The join post and the dial-back answer a stranger, each against the
     pending request rather than the member list, and refuse one once it has
     expired.
   - Removing a member takes effect on the next call.
2. **The join request and the pending row** — Add on A, the request held on
   B with its expiry, A's pending row with fingerprint and Cancel.
   - A cancelled request is gone from B; an expired one reads so on A.
3. **The confirmation modal and its push** — a nudge kind for a pending join,
   the modal in the workbench, a push notification titled for the device.
   - Allow and Deny each settle the request once; a second workbench sees the
     modal go.
4. **The exchange and the announcement** — B dials back, verifies, hands over
   its members, and announces A to each of them over its own link; every member
   records A from B rather than from A.
   - After a join, `GET` Devices on any of three servers lists the same
     three.
   - A device that announces itself to a member is refused; only a member's own
     peers can name a newcomer to it.
5. **Addresses in order, and unreachable** — the address list on every
   exchange, tried in order with a short timeout; a member that answers
   nothing dims.
   - A member moved to a new IP is reached on the next call by its advertised
     list.
6. **Unlink** — asked once, broadcast to every member, the leaver told to
   forget everyone.
   - The leaver's Devices list is empty but for itself.

## Re-verify at start

- Stage 01 landed: the peer listener, the identity endpoint and the Devices
  section of the Remote access pane exist, and the membership check is per
  route with something to consult.
- The nudge stream is still `crates/server/src/nudge.rs` with payload-less
  kinds in `crates/schema/src/nudge.rs` and the invalidation table in
  `web/src/nudge.ts`; a join needs a kind that carries the request id or the
  client re-reads a pending list.
- The one modal is still `web/src/Modal.tsx` with the confirm pattern in
  `web/src/repos/RepoList.tsx`.
- Push is still `crates/server/src/push.rs` with `News` as a closed enum.
- The Remote access pane's own stance — nothing confirmed twice, everything
  read rather than configured — is what the Devices section sits inside and what
  its Unlink deliberately departs from, as Remove on a Repo does.
