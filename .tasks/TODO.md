# The Keeper and its Annex across a network move

The review of *The Annex and its Keeper* found that a Keeper and its Annex can
lose each other for good, until the Keeper next starts, whenever the Keeper's
own addresses move: a laptop changing networks, or going offline. Both ends
dial each other only at addresses that came off the network, and the one
address that always works — the one the Annex Forward points at — is never
dialled. This is the work of making each end reach the other whatever the
Keeper's network does.

## Tasks

- [x] 01: Reach each other without the network — [details](01-reach-each-other-without-the-network.md)
