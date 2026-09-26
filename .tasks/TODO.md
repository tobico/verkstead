# Ranks

The sidebar's order becomes a **Rank** per Conversation — a fractional-indexing
key with the device that issued it suffixed after it — rather than a dense
integer rewritten across the whole table on every drag. A new Conversation is
ranked above everything at the moment it is started, so there is no unranked
state and the *unplaced float to the top* rule goes; a migration ranks every
existing row in the order the sidebar shows it in today. A drag or an arrow key
saves one row rather than the list, which is what will let stage 06 write a
drag on the merged list to the owning device alone.

Demonstrable end to end on one device: drag a row, reload, restart — the order
holds, and the request that saved it carried one Conversation.

Roadmap stage: [05: Ranks](docs/roadmaps/cluster-mode/05-ranks.md)

## Tasks

- [x] 01: Every Conversation carries a rank — [details](01-every-conversation-carries-a-rank.md)
- [x] 02: The order is the rank, and a drag writes one row — [details](02-the-order-is-the-rank.md)
- [ ] 03: Retire the whole-list order — [details](03-retire-the-whole-list-order.md)
