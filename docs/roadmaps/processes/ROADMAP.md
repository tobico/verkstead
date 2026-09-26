# Processes roadmap

A Conversation gets a **Process** — Develop, Investigate, Review, Tinker or Fix
Merge Issues — picked on the composer beside the Repo and frozen when the work
starts, saying up front what kind of work this is and so which states it runs
through. The composer's setup row becomes **Repo**, **Process**, **Agent**, the
three role pickers folding into one Agent control shaped by the Process. The
decisions and their why are in [ADR-0020](../../adr/0020-a-conversation-has-a-process.md);
the terms are in [CONTEXT.md](../../../CONTEXT.md) under **Process** and
**Investigating**, and the revised **Conversation**, **Worktree**, **Brief**,
**Pairing**, **Adopt**, **Follow-up**, **Nothing else**, **Steer**,
**Mergeable** and **Done signal** entries. The briefs reference both rather
than restating them.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

Stage 01 is the record and the picker, and everything after it stands on it.
Stage 02 is the Agent control, placed second so that every multi-role Process
lands with the control that configures it. Stages 03, 04 and 05 depend on 01 and
02 — each asks its Process for an Agent control of the right shape — and are
reorderable among themselves; 03 is the one that retires *No grilling*, so until
it lands the row stays. Stage 06 depends on 05, whose target naming and take-up
at Start it reuses, and stage 07 on 06: the stack is split off because recording
several pull requests in one repository rekeys five tables through
`migrations.rs`, which is a feature beside the narrowed wrap-up rather than the
tail of it. The Process picker offers a Process only once its
stage has landed, so the picker grows a row per stage — Fix Merge Issues from
06, over one pull request, and the stack arriving under it.

## Stages

- [x] 01: Process on the record — [brief](01-process-on-the-record.md)
- [x] 02: The Agent control — [brief](02-the-agent-control.md)
- [x] 03: Tinker, and No grilling retires — [brief](03-tinker.md)
- [x] 04: Investigate — [brief](04-investigate.md)
- [ ] 05: Review — [brief](05-review.md) *(in progress: `roadmaps/processes/05-review`)*
- [ ] 06: Fix Merge Issues — [brief](06-fix-merge-issues.md)
- [ ] 07: A stack of pull requests — [brief](07-a-stack-of-pull-requests.md)
