# Steer roadmap

Verkstead's control model collapses to one stop and one wheel: a Conversation
is driven or stopped, and **Steer** — one modal — moves it into any state by
hand, recreating what is missing and resuming in the same press. The decisions
and their why are in [ADR-0010](../../adr/0010-one-stop-and-steer.md); the
terms are in [CONTEXT.md](../../../CONTEXT.md), which each stage updates as it
retires or renames them.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

Not reorderable: stage 02 clears the one stop stage 01 builds, and stage 03
retires nothing until stage 02's Steer can replace it.

## Stages

- [x] 01: One stop — [brief](01-one-stop.md)
- [ ] 02: Steer — [brief](02-steer.md) *(in progress: `steer`)*
- [ ] 03: Close and the retirements — [brief](03-close-and-retirements.md)
