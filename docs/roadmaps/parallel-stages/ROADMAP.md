# Parallel stages roadmap

A roadmap **declares which stages stand on which**, on the stage's own line, and
Verkstead starts every stage whose dependencies have settled instead of the one
lowest unticked box — up to a limit per roadmap and one across the server. The
stages still leave one stack of pull requests behind: a stage **joins the
chain** as it finishes, before its pull request opens, so nothing anybody has
started reading is rewritten. The decisions and their why are in
[ADR-0021](../../adr/0021-parallel-stages.md); [CONTEXT.md](../../../CONTEXT.md)
gains and revises its terms stage by stage as each lands — **Stage** and
**Adopt** above all. The briefs reference the ADR rather than restating it.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

This is the first roadmap written in the form it builds, and the declarations
on the lines below are the dependencies. 01 and 02 stand on nothing and are
reorderable with each other. 03 stands on 02: the wait to join is a wait on
stages *settling*, which is the record 02 keeps. 04 stands on 01 for the
declarations it schedules by and on 03 for a safe way to finish what it starts
side by side — which is why the join is built before the scheduler, and why 03
changes nothing anybody can see in a roadmap run in order. 05 and 06 both stand
on 04 and are reorderable with each other.

**These stages themselves run in order.** The server running this roadmap is
the installed one rather than this branch, and to it what follows a link is
text it keeps and does not read. Once 04 has landed and been released, whatever
is left runs by its declarations.

## Stages

- [ ] 01: Dependencies on the record — [brief](01-dependencies-on-the-record.md) — no dependencies
- [ ] 02: The server's record of a stage — [brief](02-the-servers-record-of-a-stage.md) — no dependencies
- [ ] 03: Joining the chain — [brief](03-joining-the-chain.md) — after 02
- [ ] 04: The scheduler — [brief](04-the-scheduler.md) — after 01, 03
- [ ] 05: The server-wide limit — [brief](05-the-server-wide-limit.md) — after 04
- [ ] 06: Every stage in flight — [brief](06-every-stage-in-flight.md) — after 04
