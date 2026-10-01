# Port forwarding

A server started in a Code pane terminal on another device of the cluster is
reachable on this device's own `localhost`, at the same port, for as long as
the terminal's tab is open here. The member holding the terminal notices the
port — a listening TCP socket held by a process in the terminal's tree — and
says so; the device whose browser has the tab attached opens a loopback
listener and carries each connection over the link as one upgrade, bridged
byte for byte the way a terminal's own socket is. The sidebar's foot reads
**Forwarding 3 ports** beside *Show archived*, and a press lists each Forward
as a link, with the device and the Conversation it belongs to.

The decisions are the grilling's (Sets 995, 997, 999 and 1000): terminals only,
never the agent's session; the attaching device forwards and no other, on its
loopback alone, the same number or skipped and said; detection on the member
while an attach over the link is held, polled, with a printed `localhost` URL
hurrying the read and forwarding nothing by itself; a member connects only a
port it is reporting to a device that holds an attach on that terminal; nothing
is stopped by hand. **Forward** is the term, kept apart from the Relay.

## Tasks

- [x] 01: A terminal's ports, read on Linux — [details](01-a-terminals-ports-read-on-linux.md)
- [x] 02: The member connects a reported port — [details](02-the-member-connects-a-reported-port.md)
- [x] 03: The hub forwards — [details](03-the-hub-forwards.md)
- [x] 04: The sidebar item and its popup — [details](04-the-sidebar-item-and-its-popup.md)
- [x] 05: The macOS and Windows readers — [details](05-the-macos-and-windows-readers.md)
- [x] 06: A printed URL hurries the read — [details](06-a-printed-url-hurries-the-read.md)
- [x] 07: The docs — [details](07-the-docs.md)
