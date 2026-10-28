# Profiles by uuid, bound per device

An Agent Profile becomes one **Replicated record** that every device holds, and
everything that names a Profile names it by its uuid. The account paths stop
being part of the record: they are a **binding** of one device, a replicated row
per Profile per device. So the Profile card can say where the Profile is bound,
and a device without a binding runs from a copy under its Data Directory, named
by the uuid. The mirror rows are bridged onto the records, not dropped, so
every Pairing survives the switch. The device marks go.

A session on a device the Profile is bound on runs exactly as today. A session
on an unbound device fetches the login from a reachable bound device, the way it
fetched from the home before, until stage 03 replaces that with the replicated
login. At the switch, a device that has members names its own unnamed Profiles
after itself. So does a device the first time it links. The four Claude rows of
a four-device cluster therefore read apart in every picker. A second binding of
one record is stage 03's press, not this stage's.

Roadmap stage: [02: Profiles by uuid, bound per device](docs/roadmaps/cluster-owned-records/02-profiles-by-uuid-bound-per-device.md)

## Tasks

- [x] 01: The uuid everywhere — [details](01-the-uuid-everywhere.md)
- [x] 02: The mirrors bridged — [details](02-the-mirrors-bridged.md)
- [x] 03: The record and the binding, here — [details](03-the-record-and-the-binding-here.md)
- [x] 04: Replicated across the cluster — [details](04-replicated-across-the-cluster.md)
- [x] 05: Launch on an unbound device, brokenness per device — [details](05-launch-on-an-unbound-device.md)
- [x] 06: The card's line per device — [details](06-the-cards-line-per-device.md)
- [x] 07: Naming the unnamed — [details](07-naming-the-unnamed.md)
