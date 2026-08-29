# Agent backends roadmap

Verkstead's sessions can run on Codex, Grok Build and OpenCode as well as
Claude Code, each at full parity: launchable under a Profile of its type,
driven through every state, asking on the channel it can afford, with a
Transcript reader and a usage-limit phrase of its own. The decisions and
their why are in [ADR-0011](../../adr/0011-agent-backends.md); the terms are
in [CONTEXT.md](../../../CONTEXT.md), which each stage updates as its piece
lands.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

Partly reorderable: 02 needs 01, and 03 needs both. 04 and 05 come after 03
in either order — 05 leans on 02 only for screen idling, its ask staying the
blocking one.

## Stages

- [ ] 01: Foundations — [brief](01-foundations.md)
- [ ] 02: Asking and idling — [brief](02-asking-and-idling.md)
- [ ] 03: Codex — [brief](03-codex.md)
- [ ] 04: Grok Build — [brief](04-grok-build.md)
- [ ] 05: OpenCode — [brief](05-opencode.md)
