# 01. The Annex and its Keeper

## Goal

Two servers on Linux, B's data directory readable by A — which is what WSL
gives a Keeper, and what a test arranges with a directory. A is told B is its
Annex, by an operation of the server's own that names B's data directory and
peer address: A reads B's `device.pem`, pins it, and posts the **Keeper's
Join**; B, started with A's id and fingerprint in its config, accepts it with
no request held and no modal, and records A as its Keeper. A's Devices list
B as *Linux* with the Annex mark and no Unlink. A third server C, announced
to by A, lists B the same way — mark, no Unlink — and reaches B at A's
address on **8424**, the forward, because B advertises A's addresses at that
port. Unlinking B is refused on every device by name; B refuses to unlink A.
B never raises its setup wizard. Demonstrable end to end with three servers.

## Decisions in force

- **A Keeper's Join is distinct from a regular join**
  ([ADR-0024](../../adr/0024-the-annex.md), *The Keeper's Join*). The Keeper
  writes its Device Id and fingerprint into the Annex's `config.yaml` as
  `kept_by` before the Annex first starts; a join presenting that fingerprint
  is accepted without a held request and without a nudge; any other join is
  held and asked about as ADR-0020 says. Driving the ordinary join over both
  workbench keys was rejected by the human: the record should say the
  membership is the Keeper's.
- **The Keeper pins before it dials.** It reads `device.pem` out of the
  Annex's data directory and dials the join pinned to that certificate — the
  one join that never accepts *whatever is there*. The trust is the human's
  compare-by-eye, done by the one party that can read both files.
- **The mark travels with the roster.** The exchange and every announcement
  carry *Annex of <device>* beside the newcomer, the members table keeps
  it, and the Devices row draws it: the OS word and hostname as before, the
  mark beside them, and no Unlink. The unlink route refuses an Annex by name
  on every device, and an Annex refuses to unlink its Keeper. A Keeper
  forgetting its own Annex is stage 04's, through a path that is not Unlink.
- **The forward.** The Keeper listens on 8424 — a constant beside
  `PEER_PORT`, with a flag and an environment variable the way the peer
  listener has — and forwards raw TCP to the Annex's peer listener. TLS stays
  end to end on the Annex's certificate; the forwarder never terminates it.
  One forward per Annex is enough for the first version; a Keeper with two
  Annexes is a later port each.
- **Advertise as told.** An Annex's Device Reading advertises its Keeper's
  addresses at the forward port rather than its own interfaces, asked from
  the Keeper over a peer route at each reading — a laptop's addresses move,
  and the Keeper's reading already keeps up with them. An Annex is never
  advertised over mDNS and never discovered.
- **No Onboarding Mode on a kept Annex.** A `kept_by` in config skips the
  startup verdict: the mode only redirects a viewer nobody opens, and a mode
  nothing will clear is worse than none.
- **The vocabulary lands here**: **Annex**, **Keeper**, **Keeper's Join**
  and the **Annex Forward** in CONTEXT.md, with *host*, *guest*, *VM* and
  *managed device* under *Avoid*.

## Proposed tasks (provisional)

1. **`kept_by` in config, and what it switches off** — the Keeper's id and
   fingerprint read at start; Onboarding Mode never comes on; the Devices
   section on the Annex draws its Keeper's row with the mark and no Unlink.
   - A server started with `kept_by` opens on the compose page, not `/setup`.
   - Its unlink route refuses the Keeper by name.
2. **The Keeper's Join** — the join route accepting a certificate matching
   `kept_by` without holding a request, recording the Keeper marked as such;
   the exchange carrying the mark; the members table and `LinkedDevice`
   gaining it.
   - A join from the kept fingerprint settles in one round trip with no
     nudge and no push; one from any other is held and asked about as before.
   - After it, both Devices lists show the relation.
3. **The mark on the roster** — announcements carry it, members record it,
   every Devices row draws it, and Unlink is refused on it everywhere.
   - Three servers: C, told by A, draws B with the mark and no Unlink.
4. **The forward and advertise-as-told** — the listener on 8424 forwarding
   to the Annex's peer address; the peer route the Annex asks its Keeper's
   addresses from; the Annex's reading advertising them at 8424; discovery
   leaving an Annex out.
   - C dials B at A's address on 8424 and the handshake is B's certificate.
   - B's identity endpoint lists A's addresses at 8424 and none of its own.
5. **Linking an Annex, as an operation** — given a readable data directory
   and a peer address: read and pin the certificate, post the Keeper's Join,
   start the forward; idempotent, so a Keeper can run it at every boot.
   - Run twice, it leaves one member row and one forward.

## Re-verify at start

- Assumes the join is still held in `peer/joining.rs` with the pinned
  certificate and the ten-minute expiry, and the dial-back in
  `peer/exchange.rs`.
- Assumes the members table in `crates/store/src/members.rs` still carries
  `renewing_from` and `acknowledged`, so the mark is one more column added
  the way `reachable` was.
- Assumes `Reading::advertising` in `device/reading.rs` is the one place
  addresses are chosen, and `discovery.rs` the one place a device is
  advertised.
- Assumes `config.yaml` is read by `settings.rs` at start and nowhere else
  needs to see `kept_by`.
- Assumes the Devices row is `LinkedDevice` in `crates/render/src/device.rs`
  and `web/src/settings/Remote.tsx` draws it with its Unlink.
