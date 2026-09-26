# 03. Retire the whole-list order

## What to build

The dense integer and everything that served it go: the route that took a whole
`order` of Conversation ids, the store function behind it, the placements table
and the module that declared it.

**The table is dropped rather than left standing**, which puts two rewrites in
one order that has to hold: the rank rewrite reads `placements`, so it runs
before the drop, and a database made fresh after this task never had the table at
all — so the rank rewrite has to read a missing table as *nothing to rewrite*
rather than as a failure.

**A Conversation's delete stops walking it.** That walk is held against the
schema by a test that asks SQLite which tables reference a Conversation and
insists every one of them is in the walk's list, so taking the table out of both
is what keeps that test passing.

**The tests come across rather than being deleted.** What the store's placement
tests were about — a stale id, an id arriving twice, an order naming only some
of the list — is re-asked of the one-row reorder where it still means something
and dropped where it does not: an order naming half the list is not a thing that
can be sent any more. The server's own test of the order route and the viewer's
drag suite likewise assert on the one-row request.

**And the prose goes with the code.** The placements module's doc argued it stood
beside the Conversations because there was no migration machinery — which stopped
being true some rewrites ago and is what made a column on the row the right
place. The sidebar component and the list query both explain the order as a whole
list arriving and being written down; that is no longer what happens.

## Acceptance criteria

- [ ] Nothing in the tree reads or writes `place`, and the whole-list order
      route is gone rather than accepting a request nobody sends.
- [ ] A database written before this stage still opens with the same visible
      order — the rank rewrite runs before the drop, and one made fresh never
      had a placements table to read.
- [ ] A Conversation's delete still empties every table that names it, held by
      the test that reads the walk against the schema.
