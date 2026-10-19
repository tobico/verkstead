# 02. Everything a member offers, read back after a restart or a blip

## What to build

One mechanism that carries a Forward across a hub restart, a hub laptop
sleeping and a link dropping (grilling of 2026-10-19, Q6 and the comment
beside it).

**A member answers everything it offers the calling device in one reading**:
every Conversation's offered terminals and their ports, over the link alone
and refused by name to a browser on the machine itself, the way the
per-Conversation ports reading is. The per-Conversation reading stays for the
`ports` Nudge, which names a Conversation.

**The hub reads it at the moments its knowledge may be stale**: whenever its
Nudge stream to a member is taken up, which the stream's *everything* Nudge
already marks and which the hub's own start does for every member; and
whenever a member a turn found not answering answers again. The result is
reconciled the way a Conversation's reading is today — a port offered and not
held is taken, one held and not offered starts its grace — across every
Conversation at once, and a Conversation the member no longer offers anything
on has its Forwards start their grace too.

**The hub's counting of the terminal attaches it relays goes.** It existed so
that a Forward never taken up, or ended, while its tab stayed open could be
read again; the everything reading covers that and more. Remove the count,
its hook in the relay's bridge, and the parsing of the attach path, and let
the hub's loop look only at what it holds.

**A member's own restart ends its terminals**, so it answers with nothing and
its Forwards end after the grace. Persisting Forwards in the store was
rejected: the member is the record of what is still listening.

In this task a member that stops answering still has its Forwards ended at
once, as today, and taken up again through the everything reading when it
answers — that is what task 03 changes. The ports suite's test of a member
not answering should keep passing through this task with its tab closed
rather than open, which is the point.

## Acceptance criteria

- [ ] A hub served afresh while a member still offers a port holds the
      Forward again within a few seconds of its start, nobody having opened a
      tab.
- [ ] A member's Nudge stream dropping and coming back, with no tab open,
      leaves the Forward standing or retaken, connections reaching the server.
- [ ] A member restarted answers the everything reading with nothing, and the
      Forwards it had end within the grace and a turn.
- [ ] The everything reading is refused by name to a browser on the member
      itself.
