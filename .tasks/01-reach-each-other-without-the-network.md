# 01. Reach each other without the network

## What is wrong

Stage 01 of the WSL roadmap (ADR-0024, *The Keeper forwards for its Annex*)
links an Annex by the Keeper's Join and has the Annex advertise its Keeper's
addresses at the Annex Forward's port (8424). That serves a **third** device
well. But the Keeper and the Annex use the same network-derived addresses to
reach **each other**, and nothing refreshes them when the Keeper's addresses
move.

**The Keeper's dials to its Annex.** `Devices::keep`
(`crates/server/src/device.rs`) dials the Annex at the address it was given,
then refreshes the Annex's member row with `Peers::identity`. That refresh,
like every successful dial, overwrites the row's addresses with what the Annex
advertises — which is the Keeper's *own* addresses at 8424 (see
`Reading::as_told` / `Reading::forwarded` in
`crates/server/src/device/reading.rs` and `annex_forward::forwarded` in
`crates/server/src/peer/annex_forward.rs`). From then on the Keeper reaches its
Annex by dialling itself at `<own LAN or tailnet address>:8424`, through its
own forward. So:

- **Offline**, the Keeper's reading has no addresses (loopback is never
  advertised), the Annex advertises an empty list, and the Keeper's row for
  its Annex has nothing to dial.
- **After a network move**, the row holds the old addresses, which are no
  longer the Keeper's, and every dial fails. Nothing writes better ones until
  `keep` runs again at the next boot (stage 03 runs it at every start).

**The Annex's dials to its Keeper.** The Annex's row for its Keeper holds the
addresses the Keeper's Join carried (`joining::kept` in
`crates/server/src/peer/joining.rs` records them from `saying`). Nothing on
the Keeper's side tells the Annex when they move, and the Annex can't reach the
Keeper to ask. So after a move the Annex can't ask what to advertise
(`Peers::forwarded_by`), so it advertises a stale answer to everyone. It also
can't announce to its Keeper or fetch a Profile mirror's login from it before a
launch. Under a Keeper's Profile, that last one means sessions in the Annex
fail to start.

The address the forward points at (`Pointing` in `annex_forward.rs`, set by
`Devices::keep`) is the Annex's direct peer address — its NAT address under
WSL's NAT, its loopback under mirrored networking — and it works whatever the
Keeper's network does. No dial uses it.

## What to decide, then build

This is a design question first, so settle it before building, in an ADR note
or an amendment to ADR-0024:

1. **How the Keeper reaches its Annex.** The likely answer is that a Keeper
   dials an Annex it keeps at the forward's target before (or instead of) the
   row's advertised addresses. That means `Peers` (`crates/server/src/peer/dialling.rs`)
   knowing the `Pointing`, or the Keeper holding the direct address on the row
   in a way no refresh overwrites.
2. **How the Annex reaches its Keeper.** It needs an address for its Keeper
   that does not move with the Keeper's network: the NAT gateway (the Windows
   host) under NAT, the loopback at the Keeper's peer port under mirrored
   networking. Which mode applies is stage 03's probe to say. Decide whether the
   Keeper hands that address over in the Keeper's Join (and in `kept_by`, which
   it already rewrites at every link), or whether the Annex works it out.
3. **When the Annex learns the Keeper's addresses have moved**, so that what
   it advertises to third devices follows the Keeper without a restart. One
   candidate is the Keeper re-running its link (or a lighter telling) when its
   own reading's addresses change. Another is the Annex re-asking
   `forwarded_by` over the address from (2) at each reading, which it already
   does once the Keeper is reachable.

This touches stage 03 of the WSL roadmap (`docs/roadmaps/wsl-annex/03-making-the-annex.md`),
which reads the Annex's address and runs the link at every boot. Keep whatever
is decided consistent with its brief, and say there what it now relies on.

## Acceptance criteria

- [ ] The decision is written down (an ADR-0024 amendment or a new ADR).
- [ ] A Keeper whose own advertised addresses are empty still reaches its
      Annex: identity, announcements and the workbench's member calls all
      succeed (a suite with a `Reading` advertising nothing for the Keeper).
- [ ] A Keeper whose addresses change after the link still reaches its Annex,
      and the Annex still reaches its Keeper (`forwarded_by` and a member call
      both succeed), with neither server restarted.
- [ ] After that change, the Annex's identity endpoint advertises the Keeper's
      *new* addresses at the forward port, within one reading hold, and a third
      device told about the Annex records them.
- [ ] Nothing a third device is told includes the Annex's direct (NAT or
      loopback) address. That stays the Keeper's alone (ADR-0024).
