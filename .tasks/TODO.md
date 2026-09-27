# The server's record of a stage

Verkstead knows from its own record **which stage of its roadmap each stage
Conversation is, and whether it has settled**, and decides what a roadmap has
left from that rather than from the ticked boxes in whichever worktree it
happens to be reading. The record is ADR-0017's extended: beside *which
roadmap*, *which stage*. Where there is no record — a roadmap worked by hand or
by the old tools — the boxes still count, and a ticked box is a stage done.

**A roadmap still runs in order, and run in order it behaves as it did.** What
changes is where the answer to *is stage 03 done* comes from, and that each
stage ticks only its own box, in its own finish commit: the `next-stage` skill
stops ticking the stages above it, and the one-stage-behind rule is retired
from the words that state it.

Roadmap stage: [02: The server's record of a stage](docs/roadmaps/parallel-stages/02-the-servers-record-of-a-stage.md)

## Tasks

- [x] 01: Which stage, written down — [details](01-which-stage-written-down.md)
- [ ] 02: Done by the record — [details](02-done-by-the-record.md)
- [ ] 03: The adoption agrees — [details](03-the-adoption-agrees.md)
- [ ] 04: A stage ticks itself, and the words — [details](04-a-stage-ticks-itself.md)
