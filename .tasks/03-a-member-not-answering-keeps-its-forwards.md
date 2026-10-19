# 03. A member not answering keeps its Forwards

## What to build

A member that stops answering — asleep, off the network, the internet gone
for a while — no longer ends its Forwards (grilling of 2026-10-19, Q4).
Ending them at once and taking them up again worked while a tab held them;
with nothing holding them, the hub keeps them itself.

**A new standing, beside forwarding and skipped: unreachable.** The listener
stays bound, so `localhost` at that port is still this Forward's and nobody
else's; a connection made to it fails, as it did before, because the member
cannot be dialled. On every turn the hub looks over, a Forward whose member is
not answering moves to unreachable, and one whose member has come back is
read again through the everything reading of task 02 — taken up where the
port is still offered, and ended through the ordinary grace where it is not.

**An unlinked member is different**: no member at all is nothing to wait for,
and its Forwards end at once, as today.

**The popup says it.** The row reads *device unreachable* under the port,
dimmed and not a link, the way a skipped row is drawn — a link would open a
`localhost` page that fails, and the row is there to say why. It is a link
again the moment the member answers. Follow the copy guide for the words;
the skipped rows' *port busy here* is the model. The rendered types are
shared with the viewer, so regenerate them.

A hub laptop sleeping is this case seen from the other side: its own server
and listeners survive the sleep, and on waking it finds the member not
answering for a while, then answering, and reads everything it offers.

## Acceptance criteria

- [ ] A member that stops answering leaves its Forward in the hub's reading as
      unreachable, the port still bound at the hub's loopback, and a `forwards`
      Nudge says the standing moved.
- [ ] Once the member answers, the Forward is forwarding again without any tab
      having been opened, and a connection reaches the server.
- [ ] A member that answers again with the port gone has the Forward end
      within the grace and a turn; an unlinked member's Forwards end at once.
- [ ] The popup draws an unreachable row dimmed, not a link, reading *device
      unreachable*, and the viewer's test of the popup covers it.
