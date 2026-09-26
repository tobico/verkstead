# 02. Browsing and the Discovered list

## What to build

Devices nobody has typed appear under **Discovered**, inside the **Devices**
section of the **Remote Access** pane. This device browses
`_verkstead._tcp.local` with the same crate task 01 advertises through, keeps
what it finds in memory keyed by **Device Id**, and answers it on a reading of
its own — apart from the Devices reading rather than a field of it, so that
finding a device does not re-read the membership and the rows the pane already
drew cannot be lost to a browse.

A row is what the TXT record and the resolved address say: the name, the mark
for the OS, the address it was found at, and the source, which reads *LAN*.
Every row carries an **Add**, which task 04 is what wires up.

**Three kinds of device are left out.** A **Member** is not drawn, because it is
in the cluster already and a row offering to link it would be a press with
nothing behind it. This device is not drawn, because it hears its own
advertisement. And a device this one holds a pending **Join** for is not drawn,
because the pending row under the list is already the answer to the press
somebody made.

**The browse is held while somebody is looking.** It starts when the Discovered
reading is first asked for and is dropped once nothing has asked for it for a
spell — a phone that closes a tab says nothing, so what keeps the browse alive
is the reading being read rather than the pane announcing itself. The spell is
long enough that a pane left open keeps its browse.

**And a browse finds things after the fact.** A cold one has heard nothing yet,
so the first read is empty or short and the rows arrive over the seconds after
it. That is what the **Nudge** is for: a kind of its own, raised when the found
list changes, so an open pane draws a device appearing or leaving without a
reload and without a poll — which is the arrangement ADR-0009 put every other
list in this viewer on.

## Acceptance criteria

- [ ] Two servers on one LAN each draw the other under Discovered — name, OS mark, address, *LAN* — within seconds of the pane being opened and without a reload.
- [ ] This device, a Member and a device with a Join pending are each absent from the list, with everything else about them unchanged.
- [ ] A device that stops advertising leaves the list within its TTL, and one that says goodbye leaves it at once.
- [ ] The Discovered list is a reading of its own: drawing it again does not re-read the membership, and a browse that found nothing leaves the Devices rows alone.
