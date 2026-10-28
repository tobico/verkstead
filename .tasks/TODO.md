# Records are the cluster's — Authors first

A replicated-table facility, and Authors as the first record kind on it. An
Author made on any device of the cluster is on every other device once it is
reachable, carrying the same uuid; an edit or a removal made anywhere lands
everywhere, a device that was off catching up when it comes back; an edit that
arrives after a removal stays removed. Nothing is relayed to a home and no row
carries a device mark.

Removing an Author that Repos point at names each Repo with its device, across
this device and every reachable member, and offers *Repoint them all to…*. The
facility is shaped for the kinds that follow — Profiles and their bindings
(stage 02), the Instructions text and MCP declarations (stage 05) — as one
thing with record kinds on it ([ADR-0028](../docs/adr/0028-records-are-the-clusters.md)).

Roadmap stage: [01: Records are the cluster's — Authors first](docs/roadmaps/cluster-owned-records/01-records-are-the-clusters.md)

## Tasks

- [x] 01: Authors are records, here — [details](01-authors-are-records-here.md)
- [ ] 02: Replicated across the cluster — [details](02-replicated-across-the-cluster.md)
- [ ] 03: Tokens from whoever holds one — [details](03-tokens-from-whoever-holds-one.md)
- [ ] 04: The Git section without homes — [details](04-the-git-section-without-homes.md)
- [ ] 05: Removal across devices and Repoint them all — [details](05-removal-across-devices-and-repoint-them-all.md)
