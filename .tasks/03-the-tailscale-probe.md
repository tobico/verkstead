# 03. The Tailscale probe

## What to build

Two Verksteads on one tailnet find each other without being on one LAN. The peer
list comes out of `tailscale status --json`, which this server already runs for
the **Remote Access** pane and for the tailnet half of the **Device Reading** —
but the reading of it parses the backend state and this machine's own node and
nothing else, so the peers and whether each of them is online are new. Every
online peer is then asked for its identity on the peer port, over the **Peer
Listener**'s own un-gated identity endpoint, taking whatever certificate it
presents for the one call the way a **Join** takes one.

Read when the Discovered reading is asked for rather than on a schedule, and
merged with the mDNS findings by **Device Id**: a device found both ways is one
row whose source says both, and one found only here reads *Tailscale*. The same
three exclusions task 02 makes apply to what the probe finds, for the same
reasons.

**Bounded, because a tailnet is somebody else's size.** The probes go out in
parallel with a short deadline apiece, a ceiling on how many are in flight and a
ceiling on how many peers are asked at all: a tailnet of a few hundred nodes must
not turn one opened pane into a few hundred TLS handshakes. Nothing about a peer
that is not a Verkstead is drawn — it refuses, it answers something else, or it
answers nothing, and all three come to the same absent row.

**And the peer port is assumed over the tailnet.** An advertisement carries the
port a device really bound; a peer list carries no port at all, so a device told
to listen somewhere else is found on the LAN and not on the tailnet. That is a
known limit of the same kind as Windows and its WSL, and the typed address is
what is left.

**And having no Tailscale is an answer rather than a failure.** A machine with
none of it, one whose daemon is down and one that does not answer inside the
deadline each leave the mDNS half of the list standing, which is the stance every
other reading of this daemon already takes.

## Acceptance criteria

- [ ] A tailnet peer running a Verkstead is drawn under Discovered with source *Tailscale*; one that answers nothing, or answers something that is not a device identity, is not drawn at all.
- [ ] A device found by both mDNS and the probe is one row, and its source reads both.
- [ ] No `tailscale`, a daemon that is down, and one that never answers each cost the list nothing but the Tailscale half.
- [ ] A status naming far more peers than the ceiling asks no more than the ceiling, each with a deadline of its own.
