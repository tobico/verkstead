# 01. The peer-side API

## What to build

The viewer's own namespace, `/api/ui/`, answered on the **Peer Listener** as
well as on the workbench one — behind the **Member Gate** there instead of the
Workbench Key's gate here. A member that has linked to this device can read and
drive its workbench over the link it already holds, presenting nothing but its
certificate, and this device's Workbench Key stays where it is.

One router of routes rather than two: the same namespace, built once, mounted
twice behind two different gates. What differs is who gets through.

**Three prefixes are this device's own and are not served over that listener at
all** — `/api/ui/remote/`, `/api/ui/devices/` and `/api/ui/push/` — refused
there by name rather than quietly missing, so a caller can tell *this is not
relayed* from *this Verkstead is too old to have it*. That is what makes *a
member's Workbench Key never leaves it* a fact about the mechanism rather than
about which pages happen to exist: the Remote access reading carries the login
link with that key on it, and a namespace served whole would hand it to whoever
holds the other device's cookie (ADR-0020, *The opened device relays*).

**And the agents' half stays off that listener.** A session's
Conversation-scoped API answers the loopback and the named pipe, which is all a
session ever dials, and the health check is nobody's Conversation. Neither the
viewer's fallback nor `/api/v1/` belongs on a port another device reaches.

Nothing in the browser changes in this task, and nothing relays yet: what is
being built is the far end of the hop. The demonstration is a member's own call
landing on a real store and answering what the workbench would have answered.

Worth knowing before starting: the Peer Listener's router today is assembled
out of a handful of device handles and holds none of the workbench's state, so
giving it this namespace is a change to how the server is wired up at startup
rather than a gate swapped for another. The un-gated surface on that listener
stays the list of three it is — the identity endpoint, the join post and the
dial-back — and everything added here is a member's or is refused.

## Acceptance criteria

- [ ] A member's `GET /api/ui/conversations` over the Peer Listener answers what
      the workbench's own does, with no key cookie sent and none needed.
- [ ] A caller presenting no certificate, or one this device holds no membership
      for, is refused in the gate's own words — and so is a path under
      `/api/ui/` that no route answers.
- [ ] A member's call under `/api/ui/remote/`, `/api/ui/devices/` or
      `/api/ui/push/` is refused by name, and the refusal says it is a namespace
      this device keeps to itself.
- [ ] `/api/v1/`, the health check and the viewer's fallback are not reachable
      over the Peer Listener.
