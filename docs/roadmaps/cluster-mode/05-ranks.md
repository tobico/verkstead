# 05. Ranks

## Goal

The sidebar's order is a rank string per Conversation rather than a dense
integer rewritten on every drag. A new Conversation is ranked above
everything; a drag or an arrow key writes one new rank for the moved row,
between its neighbours; a migration gives every existing row a rank in its
present order, unplaced ones on top newest first, and the *unplaced float to
the top* rule is gone. Demonstrable end to end on one device: drag, reload,
restart — the order holds, and the request that saved it carried one row.

## Decisions in force

- **Fractional-indexing strings** ([ADR-0020](../../adr/0020-cluster-mode.md),
  *Ranks*): a key between any two always exists, keys grow only where one
  keeps inserting in one spot, no bucket and no rebalance. Jira-style lexorank
  was rejected for its rebalance pass; a hub-held merged order was rejected
  because the order would differ per hub.
- **Every rank carries the device that issued it**, as a suffix after the key.
  A device computes its keys off its own list alone, so two devices ranking
  above their own top mint the same key — the first rows of a fresh device, not
  a rare coincidence — and two rows sorting equal would leave stage 06's merge
  ambiguous and a drag between them inexpressible, fractional indexing having
  no key strictly between two equal ones. Suffixed, the keys are distinct
  cluster-wide by construction and the merge needs no tiebreaker. It matters on
  one device too: the suffix is what stage 06 merges on without changing
  anything written here, so it goes in from the start rather than being added
  to every existing row later.
- **Every Conversation is ranked at creation**, above everything, so there
  is no unranked state; the migration ranks what exists in today's order.
- **A reorder is one row's new rank**, sent as one call, not the whole list.
  This is what lets stage 06 write a drag to the owning device alone.
- **The sidebar keeps its drag and arrow-key behaviour**; only what it sends
  and how the server orders change.

## Proposed tasks (provisional)

1. **The rank column and the migration** — `rank TEXT` on the placements
   shape or the Conversation row, the migration from places, the order-by.
   - A database from before the change opens with the same visible order.
2. **Rank at creation** — every start path assigns a rank above the current
   top, suffixed with this device's id.
   - Two Conversations created back to back read newest first.
   - Two devices ranking above an empty list produce keys that sort apart.
3. **One-row reorder** — a route taking a Conversation and its new rank; the
   client computes the key between neighbours and sends that.
   - Dragging to the top or bottom yields a key outside the range; dragging
     between two rows yields one between them.
   - A key computed between two neighbours sorts between them with their
     suffixes on, and the moved row keeps its own device's suffix.
4. **Retire the whole-list order** — the old route and its store function
   removed, with their tests rewritten for ranks.

## Re-verify at start

- The order is still `placements(conversation_id, place)` in
  `crates/store/src/placements.rs` with `ORDER BY m.place IS NULL DESC,
  m.place, c.id DESC` in `crates/store/src/conversations.rs`.
- The drag is still hand-rolled in `web/src/workbench/Conversations.tsx`
  sending `order: number[]` to `/api/ui/conversations/order`.
- Whether a fractional-indexing crate and npm package pair exists that agree
  on the key alphabet, or whether both ends are written here — and whether
  either will generate a key between two suffixed ones, or whether the suffix
  has to come off before the arithmetic and go back on after.
