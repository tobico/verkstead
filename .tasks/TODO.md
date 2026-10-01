# Sizes and eviction

Every language's store has a size, and stays under it. The **Language support**
pane on the settings page shows what each store holds on disk beside the size it
is allowed, with a field to change the size and a Clear. A machine that has run
Verkstead for a year has not had its disk filled by packages for projects nobody
has opened since.

Tools that evict for themselves — sccache, and only sccache — are handed the
size and left alone. For every other store the descriptor names its **unit**, and
a sweep on a timer removes whole units, oldest first, while nothing is running.
The decisions are ADR-0021's, as amended by this stage's grilling (Set 996): a
unit is named by a depth *or a marker*, a content-addressed store's unit is one
blob, Rust's cargo half is swept under Rust's size, Clear is refused while
anything runs, and a never-idle machine is never swept.

Roadmap stage: [05: Sizes and eviction](docs/roadmaps/language-caches/05-sizes-and-eviction.md)

## Tasks

- [x] 01: A size for every language — [details](01-a-size-for-every-language.md)
- [ ] 02: The stores, and their disk use — [details](02-the-stores-and-their-disk-use.md)
- [ ] 03: The unit, in the grammar and the built-ins — [details](03-the-unit.md)
- [ ] 04: The sweep — [details](04-the-sweep.md)
- [ ] 05: Clear — [details](05-clear.md)
- [ ] 06: The docs — [details](06-the-docs.md)
