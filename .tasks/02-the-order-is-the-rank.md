# 02. The order is the rank, and a drag writes one row

## What to build

The sidebar's order becomes the rank, and letting go of a dragged card saves one
Conversation rather than the whole list.

**The list is ordered by rank alone**, and the *unplaced float to the top* rule
goes with the column it was about: there is no unplaced Conversation any more,
because every start mints a rank above everything. A Conversation started while
the sidebar is open still arrives at the top — that is now what its rank says
rather than what the query does about a null.

**A reorder names the Conversation and where it now sits**, and the server mints
the key. The route takes the row it now sits directly below, with *nothing*
meaning the top of the list; the server reads that row's rank and the rank of
whatever is next below it, mints a key between the two and writes it. So the
arithmetic exists once, in one language, and the viewer never learns what a key
looks like:

    PUT  /api/ui/conversations/{id}/rank    { "below": <id> | null }

**The moved row keeps its own device's suffix.** The device that writes the rank
is the device that owns the row, which is what stage 06 merges on: a hub that
computes a key for somebody else's row will hand it to that device to write.

**The sidebar keeps its drag and its arrow keys exactly as they behave now** —
the grace distance before a press becomes a drag, the long press that lifts a
card under a finger, the local order held until the server's list agrees with
it, the error line under the list when it could not be saved. What changes is
only what letting go sends, and an arrow-key step sends the same thing.

**A neighbour that has gone since the list was drawn is not a refusal.** The row
lands where the rest of the list says, the way a stale id in the whole-list order
was passed over rather than refusing the drag it was only partly about. And the
Nudge that carried the old order to the other open workbenches carries this one.

CONTEXT.md gains **Rank** — what it is made of, why the device is suffixed, that
it is minted above everything at creation, and that it is what the sidebar
orders by.

The whole-list order route and its store function are still there at the end of
this task, with nothing calling them; task 03 is what takes them out.

## Acceptance criteria

- [ ] Drag a row to a new position, reload the page, restart the server — the
      order holds each time, and the request that saved it named one
      Conversation.
- [ ] Dropping at the top or at the bottom mints a rank outside the range;
      dropping between two rows mints one that sorts between them with the
      suffixes on.
- [ ] An arrow key moves a row the same way a drag does, and a Conversation
      started while the sidebar is open arrives at the top of it.
- [ ] CONTEXT.md defines Rank in the project's own vocabulary.
