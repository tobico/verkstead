# 02. The member connects a reported port

## What to build

A device that has read a port off a member needs a way to reach it, and the
link already carries sockets: the Relay copies an upgraded connection's bytes
blindly between a browser and a member, with either end's close crossing. This
task gives the member an upgrade of its own to answer.

**One upgrade per connection**, over the Peer Listener, naming the Conversation,
the terminal and the port. The member answers by connecting to that port on its
own loopback and joining the two: bytes crossed both ways, whichever side ends
ending both with a shutdown rather than a drop, the way the Relay's own bridge
does. Nothing in the bytes is the member's business.

**Refused by name unless the caller may have it.** The caller must be a member
holding an attach on that terminal, and the port must be in the terminal's
current reading; anything else — a port the terminal never opened, a terminal
the caller is not attached to, this device's own browser asking — is refused
with a reason, before any loopback connection is made. A port that has just
closed is refused the same way, which is the reading being the gate rather than
the loopback being one.

A connection that the loopback refuses, the server in the terminal having gone
between the reading and the dial, answers the upgrade with a refusal rather
than a socket that carries nothing.

## Acceptance criteria

- [ ] A request sent through the upgrade from another member reaches a server listening in the terminal and its answer comes back; a close at either end closes the other.
- [ ] The upgrade is refused by name for a port not in the reading, for a terminal the caller does not hold an attach on, and from the device's own browser.
- [ ] A port whose listener went away between the reading and the dial answers a refusal, not an empty socket.
