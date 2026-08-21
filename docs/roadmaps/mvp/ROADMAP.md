# MVP roadmap

Turns this askance clone into **Verkstead**, the agentic-coding management
platform designed in [docs/design/verkstead.md](../../design/verkstead.md):
conversations driven from a 3-pane workbench, sandboxed claude sessions
managed by an orchestrator, no commit gates, review consolidated in a per-PR
wrap-up phase.

Each stage is one `/to-tasks` feature (one branch, one review unit). Start the
next one with `/next-stage` in a fresh session. Task chunkings inside briefs
are provisional — re-grounded against the codebase when the stage starts.

Stages are strictly sequential: 02 executes in the sandbox what 01 records, 03
implements what 02 grills, 04 wraps what 03 lands, 05 refines all four. None
are reorderable. Adoption (replacing roadrunner and the scripts for daily
work) happens after stage 04.

The original stage 01, Skeleton, was split in two on 2026-08-20 when
re-grounding at its start found it combining a whole-repo rename with a
process supervisor — the roadmap had flagged that risk and left the call to
this point. Its halves are 01 and 02 below, and the stages after them shifted
up by one.

## Stages

- [x] 01: Workbench — [brief](01-workbench.md)
- [x] 02: Grilling — [brief](02-grilling.md)
- [x] 03: Implementation — [brief](03-implementation.md)
- [ ] 04: Wrap-up — [brief](04-wrap-up.md)
- [ ] 05: Refinement — [brief](05-refinement.md)
