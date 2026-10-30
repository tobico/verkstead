# Home drives a roadmap across devices

A stage of a roadmap can be worked on another device without the roadmap losing
sight of it. A stage's home — the device it was born on, which is where every
stage is made — keeps a tombstone for each stage moved away; the live copy tells
home every lifecycle move and home writes it onto the tombstone, so standings,
places and the chain are read where they always were. The far device's
scheduler ignores a stage whose home is elsewhere, and a finish there asks home
over the link whether it may join and what the chain's top is, waiting — said on
the Timeline — while home is unreachable.

Demonstrable: a stage transferred by hand to a second device finishes there, and
the first device starts the stage that stood on it. The decisions are
[ADR-0029](docs/adr/0029-work-is-placed-by-platform.md), *Home drives a roadmap
across devices*.

Roadmap stage: [05: Home drives a roadmap across devices](docs/roadmaps/platform-placement/05-home-drives-a-roadmap-across-devices.md)

## Tasks

- [x] 01: The report — [details](01-the-report.md)
- [x] 02: The far scheduler stands down — [details](02-the-far-scheduler-stands-down.md)
- [ ] 03: The join over the link — [details](03-the-join-over-the-link.md)
- [ ] 04: Coming home — [details](04-coming-home.md)
