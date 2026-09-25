# 02. Dialling a peer

## What to build

The outbound half of the **Peer Listener**. Stage 01 built the listener and
nothing that calls one: the only TLS client anywhere in this tree is hand-rolled
inside the peer listener's own tests, so a device that can be reached still
cannot reach. This task writes the one way this tree dials another device, and
every task after it goes through it.

**What a dial is.** This device presents its own certificate — the one the
listener presents, which over a **Changeover** is the outgoing one, because what
a member holds is what a member has to be answered with. The far end's
certificate is checked against the fingerprint recorded for that member, and a
far end whose certificate is not the one recorded is refused: there is no
certificate authority anywhere in a cluster, so the pinned fingerprint is the
whole of what proves the other end. Nothing here trusts a host name or a chain.

**Every address, in order, with a short timeout** (ADR-0020, *Addresses*). A
member's row holds every address it advertised and the order it advertised them
in — the tailnet name and its addresses first, then the LAN ones — and a dial
tries them in that order until one answers. A laptop moves between the LAN and
the tailnet and DHCP moves everybody, so the address somebody typed at link time
is only the first one ever known. The timeout is per address rather than for the
dial as a whole: a list of four addresses where the first three are gone should
still reach the fourth well inside the patience of whatever is waiting.

**Last seen, and *unreachable*.** A dial that got through writes the moment it
did against the member. A dial that found nothing at any of its addresses leaves
the row alone and marks it unreachable **on that first failure** — the human
settled this over a grace period: a row that dims the moment a machine stops
answering is a row that tells the truth about what a press on it would do, and a
laptop with its lid shut is a machine that is not there. The row stays on the
list, dimmed, reading *unreachable*; it is not removed, and nothing about it is
forgotten. It un-dims on the next dial that gets through.

**Where the addresses come back from.** A device answers for itself on the
identity endpoint, which asks the caller for nothing — so the cheapest thing a
dial can be is a read of that, and it is also what refreshes a member's name, OS
and addresses against what the machine now says. Take that as the dial this task
proves itself with, and write nothing that only a member may ask: the stage's
member-only routes arrive in tasks 06, 07 and 08.

**One more thing the dial has to handle**: the far end naming a certificate other
than the one it presented. The identity answer carries the fingerprint of the
certificate the handshake used, precisely so a caller can check the two against
each other, and a device that names one certificate and presents another is not
the device it says it is.

## Acceptance criteria

- [ ] A member whose first advertised addresses are gone is still reached on a
      later one, and the dial presents this device's own certificate.
- [ ] A member that answers at none of its addresses is left on the list, dimmed
      *unreachable*, with nothing about it forgotten — and un-dims on the next
      dial that gets through.
- [ ] A far end whose certificate is not the fingerprint recorded for that member
      is refused, and so is one that names a fingerprint other than the
      certificate it presented.
- [ ] A dial that gets through records when it did, and a member's name, OS and
      addresses are refreshed from what the far end now says.
- [ ] A list of dead addresses does not hold the caller up beyond the per-address
      timeout multiplied by their number.
- [ ] Over a changeover the dial presents the outgoing certificate, which is what
      the far end holds.
