# The merged list

One sidebar for the cluster. The device the browser opened keeps every member's
Conversation list in memory, refreshed on that member's own Nudges, and merges
it with its own by **Rank** — so the list reads the same from any device in the
cluster, every row says which device owns it, and a drag across devices writes
one rank to that device and to nobody else. A member that stops answering keeps
its rows, dimmed *unreachable*, from the last list held.

Every row's second line reads the OS icon, the device and then the repo once
anything is linked — this device's own rows too, so the list reads as one — and
the pane header and the row read aloud carry the device with it. The hub's
*Show archived conversations* switch governs the whole merged view, and a
member's push news reaches the hub's phones with the device leading the title.

Demonstrable end to end with two devices and a phone.

Roadmap stage: [06: The merged list](docs/roadmaps/cluster-mode/06-the-merged-list.md)

## Tasks

- [x] 01: Member lists in memory, merged by rank — [details](01-member-lists-merged-by-rank.md)
- [x] 02: The row, the header and the spoken row — [details](02-the-row-the-header-and-the-spoken-row.md)
- [x] 03: The hub's archived switch governs — [details](03-the-archived-switch-governs.md)
- [x] 04: The cross-device drag — [details](04-the-cross-device-drag.md)
- [ ] 05: The push relay — [details](05-the-push-relay.md)
