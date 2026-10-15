# 04. The Annex Forward

## What to build

An Annex inside WSL has an address only its Keeper can dial, so the Keeper
listens on a port of its own and forwards raw TCP to the Annex's peer listener
(ADR-0024, *The Keeper forwards for its Annex*; CONTEXT.md, **Annex
Forward**).

- **The listener.** A constant for 8424 beside `PEER_PORT`, and a flag and
  environment variable for its address the way the peer listener has
  (`--peer-listen` / `VERKSTEAD_PEER_LISTEN`), on every interface by default.
  Bound at startup beside the peer listener.
- **Raw bytes, never terminated.** Each connection accepted is joined to a
  fresh TCP connection to the Annex's peer address and the two are copied both
  ways until either closes. TLS stays end to end on the Annex's own
  certificate; the Keeper never reads a byte of it. This is new code — the
  existing `forwarding.rs` is the terminal **Forward**, carried over HTTP, and
  is not this.
- **Pointed by the linking operation.** Task 02's operation points the forward
  at the Annex's peer address it was given. With no Annex linked, a connection
  is closed at once. One forward per Annex is the first version, and a Keeper
  has one Annex; a second Annex is a later port each.
- Running the operation again re-points the one forward rather than adding
  another.

Nothing about the forward is drawn, and the NixOS module gains nothing — a
Keeper is never a NixOS daemon in the first version.

## Acceptance criteria

- [ ] After A links B, a TLS dial to A's forward port completes its handshake
      on B's certificate, and B's identity endpoint answers through it.
- [ ] Before any Annex is linked, a connection to the forward port is closed
      with nothing answering.
- [ ] Running the linking operation twice leaves one forward, pointed at B.
- [ ] The forward's address is set by its flag and by its environment
      variable, and defaults to every interface on 8424.
