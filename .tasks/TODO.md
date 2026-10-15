# The Annex and its Keeper

Two Verksteads on one Linux machine, the second's data directory readable by
the first — which is what WSL gives a Keeper, and what a suite arranges with a
directory. The Keeper (A) is told B is its Annex by an operation of its own
naming B's data directory and peer address: it reads B's `device.pem`, pins
it, and posts the **Keeper's Join**, which B — told in its `config.yaml` that
it is kept by A — accepts in the join's own reply, with no request held and no
modal. Every device's Devices list then draws B with the Annex mark and no
Unlink, Unlink is refused on it everywhere, and B never unlinks A or raises its
setup wizard.

B is reached through the **Annex Forward**: A listens on 8424 and forwards raw
TCP to B's peer listener, and B advertises A's addresses at that port rather
than its own interfaces. So a third device C, told about B by A, reaches B at
A's address on 8424 and meets B's own certificate. Platform-neutral, and
demonstrable end to end with three servers; stage 03 is what makes the box on
Windows and calls the linking operation at every boot (ADR-0024).

Roadmap stage: [01: The Annex and its Keeper](docs/roadmaps/wsl-annex/01-the-annex-and-its-keeper.md)

## Tasks

- [ ] 01: `kept_by`, and what it switches off — [details](01-kept-by.md)
- [ ] 02: The Keeper's Join — [details](02-the-keepers-join.md)
- [ ] 03: The mark on the roster, and Unlink refused — [details](03-the-mark-on-the-roster.md)
- [ ] 04: The Annex Forward — [details](04-the-annex-forward.md)
- [ ] 05: Advertise as told — [details](05-advertise-as-told.md)
