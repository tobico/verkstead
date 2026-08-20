# MVP roadmap

Turns this askance clone into **Verkstead**, the agentic-coding management
platform designed in [docs/design/verkstead.md](../../design/verkstead.md):
conversations driven from a 3-pane workbench, sandboxed claude sessions
managed by an orchestrator, no commit gates, review consolidated in a per-PR
wrap-up phase.

Each stage is one `/to-tasks` feature (one branch, one review unit). Start the
next one with `/next-stage` in a fresh session. Task chunkings inside briefs
are provisional — re-grounded against the codebase when the stage starts.

Stages are strictly sequential: 02 executes what 01 models, 03 wraps what 02
lands, 04 refines all three. None are reorderable. Stage 01 is the largest;
if re-grounding at its start says it exceeds one review unit, split it then
rather than now. Adoption (replacing roadrunner and the scripts for daily
work) happens after stage 03.

## Stages

- [ ] 01: Skeleton — [brief](01-skeleton.md)
- [ ] 02: Implementation — [brief](02-implementation.md)
- [ ] 03: Wrap-up — [brief](03-wrap-up.md)
- [ ] 04: Refinement — [brief](04-refinement.md)
