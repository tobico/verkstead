# Session output roadmap

Agent output today is raw terminal bytes dumped in a `<pre>`. This roadmap
replaces that with the two records the grilling settled on: a readable
**Transcript** built from the agent's own session log ([ADR
0006](../../adr/0006-transcript-from-session-log.md)), and a live **Screen**
the human can watch and — by taking a **Hold** — type into ([ADR
0007](../../adr/0007-server-held-terminal.md)). The terms are in
[CONTEXT.md](../../../CONTEXT.md); the decisions live in those ADRs and are
referenced, not restated, by the stage briefs.

Each stage is one `/to-tasks` feature (one branch, one review unit). Start
the next one with `/next-stage` in a fresh session. Task chunkings inside
briefs are provisional — re-grounded against the codebase when the stage
starts.

Stage 02 depends on stage 01: it replays the Capture, which stage 01 names,
and its screen sits beside the Transcript pane stage 01 builds. Not
reorderable.

## Stages

- [x] 01: The Transcript — [brief](01-transcript.md)
- [ ] 02: The Screen and the Hold — [brief](02-screen-and-hold.md)
