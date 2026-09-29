# The scheduler

A declared roadmap runs by its declarations: every stage whose dependencies have
settled starts, up to three of one roadmap at once, whenever a stage settles or
the human presses *Continue a roadmap*. A stage that halts before it has joined
the chain holds up only the stages that stand on it — its siblings carry on, and
it keeps its place. A roadmap that declares nothing runs strictly in order, as it
always has: each stage standing on the one before it is what *in order* means to
the scheduler, so there is one scheduler rather than two.

What it replaces is the rule that the next stage is the lowest unticked box,
started by the one before it settling. The guards that assumed one stage in
flight go with it — the adoption refusing a roadmap that has one, the reading
that returned a single stage, and the carry-on taking *branch already taken* as
*nothing was started*. What they protected against, a second Conversation on a
stage already under way, is still refused by the record and by the branch.
Demonstrable on a roadmap shaped like this one: 01 and 02 start together, 03
starts when 02 settles whatever 01 is doing, and 04 waits for both.

Roadmap stage: [04: The scheduler](docs/roadmaps/parallel-stages/04-the-scheduler.md)

## Tasks

- [ ] 01: What a declared roadmap has ready — [details](01-what-is-ready.md)
- [ ] 02: Read afresh off the top of the chain — [details](02-off-the-top-of-the-chain.md)
- [ ] 03: A settle starts every ready stage — [details](03-a-settle-starts-them-all.md)
- [ ] 04: The roadmap's limit — [details](04-the-roadmaps-limit.md)
- [ ] 05: Continue a roadmap starts every ready stage — [details](05-continue-a-roadmap.md)
- [ ] 06: A bad roadmap, refused at both ends — [details](06-a-bad-roadmap-refused.md)
- [ ] 07: The words — [details](07-the-words.md)
