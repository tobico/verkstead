# 03. The hub's archived switch governs

## What to build

*Show archived conversations* is one switch for the whole merged list. It is the
human's standing choice about a list rather than a setting on a machine, and the
list they are looking at is the cluster's — so the device they opened decides,
and each member is asked accordingly.

**The member is asked with the flag rather than filtered afterwards.** The hub's
read of a member's list carries the position its own switch stands at, and the
member answers accordingly **without touching its own switch**: that row is that
device's own standing choice for the browser in front of *it*, and a hub that
wrote to it would be one device changing what another one sees.

So the store's own listing stops reading the switch inside its query and takes
the position as an argument; the endpoint reads this device's switch where
nothing says otherwise, and reads what it was asked for where the call came over
the Peer Listener. And the hub re-reads every member's held list the moment its
own switch moves, because the held lists were fetched under the old position.

**Whether there is anything archived at all folds across the cluster.** That
flag is the one thing the list cannot say for itself — a filtered list is the
same empty list whether nothing was ever worked on or a hundred Conversations
stand behind a switch that is off — and it is exactly what decides whether the
switch is worth drawing. In a cluster it is *anything archived anywhere*: a
device with nothing of its own still draws the switch while a member has
something behind it. The hub holds each member's answer to that beside the list
it holds for that member, refreshed together, so reading it costs no extra call.

**And archiving reaches the owning device.** Archive, Unarchive and Close and
archive on a member's row go to that device through the Relay, as every other
row of the card's menu does after task 01 — what is left here is that the row
leaves or joins the merged list on the hub's switch rather than on the member's.

CONTEXT.md's **Archived** entry gains the cluster sentence: the standing choice
is the opened device's and governs every member's rows, and each member is asked
with it rather than told to change its own.

## Acceptance criteria

- [ ] Turning the hub's switch on brings a member's archived Conversations onto
      the merged list in their ordinary places, and turning it off takes them
      off — with the member's own switch unchanged throughout.
- [ ] A device with nothing archived of its own draws the switch while a member
      has something archived, and draws none when nothing anywhere does.
- [ ] Archiving a member's Conversation from the merged list takes its row off
      the list, and unarchiving it puts it back.
- [ ] CONTEXT.md says the switch is the opened device's and governs the merge.
