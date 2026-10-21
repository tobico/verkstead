# Previews back at the root, with a hover card

A Preview pressed a second time types its command into the shell it ran in
before, and a command like `cd web && pnpm dev` leaves that shell in `web/`,
so the second run fails on the `cd`. The server is to notice the shell has
moved and type a `cd` back to the worktree root first. And a Preview's row
is to show, on hover, what a press would run: the command, and a markdown
description the agent writes under the command when it suggests one.

Settled in the grilling of 2026-10-21 and recorded as an amendment to
ADR-0024, with the vocabulary in CONTEXT.md under **Preview**, **Terminal**
and **Code** — all three written ahead of the tasks, so each task builds to
words already there.

## Tasks

- [x] 01: A retyped Preview starts from the worktree root — [details](01-retyped-preview-starts-from-the-root.md)
- [x] 02: A hover card over a Preview's row, holding its command — [details](02-hover-card-over-a-preview.md)
- [ ] 03: A description reaches the card — [details](03-a-description-reaches-the-card.md)
- [ ] 04: The agent is told about descriptions — [details](04-the-agent-is-told-about-descriptions.md)
