# The relay

A member's whole workbench is reachable through any other member. Opening
`/devices/{device}/conversations/{id}` on A draws a Conversation that lives on
B, and everything works as if on B — the Timeline, answering a Set, the
terminal and the Code pane with their sockets, an attachment upload, the Repo
dropdown's Open and Create — with A's cookie and nothing else in the browser.
A member serves `/api/ui/` over its Peer Listener behind the Member Gate, the
device the browser opened forwards a call to `/api/ui/members/{device}/…`
verbatim and hands the answer back untouched, and a member's Workbench Key
never leaves it (ADR-0020, *The opened device relays*).

The hub holds one Nudge stream to each member and re-announces every member
nudge locally under the device, so the page stays fresh without a poll and
without a reload. Demonstrable end to end: grill a Conversation on B, and drive
it from A's browser to Done.

Roadmap stage: [04: The relay](docs/roadmaps/cluster-mode/04-the-relay.md)

## Tasks

- [x] 01: The peer-side API — [details](01-the-peer-side-api.md)
- [x] 02: The plain relay — [details](02-the-plain-relay.md)
- [x] 03: The device in the client — [details](03-the-device-in-the-client.md)
- [x] 04: The sockets — [details](04-the-sockets.md)
- [x] 05: Member nudge streams on the hub — [details](05-member-nudge-streams.md)
