# Implementation

Takes a Conversation from finished grilling to implemented work without ever
touching the CLI. The grilling agent proposes wrap-up as a final Question Set;
answering it moves the Conversation to Direction, where the agent recommends
inline / task list / roadmap and the human chooses in the GUI. From there
sessions execute unattended — committing freely, one fresh session per task —
with commits arriving as reviewable Timeline Events.

This is stage 03 of the MVP roadmap. Roadmap execution itself is stage 04; what
lands here is the choice UI plus the inline and task-list paths. There are no
commit gates anywhere in it: the agent commits on its own, and feedback
consolidates in stage 04's wrap-up phase.

Roadmap stage: [03: Implementation](docs/roadmaps/mvp/03-implementation.md)

## Tasks

- [x] 01: Wrap-up proposal and the Direction state — [details](01-direction-state.md)
- [x] 02: Handoff document and inline execution — [details](02-inline-execution.md)
- [x] 03: Commit Timeline Events — [details](03-commit-events.md)
- [x] 04: The to-tasks fork in-conversation — [details](04-task-list-path.md)
- [ ] 05: The pinned task-list Event — [details](05-pinned-task-list.md)
- [ ] 06: The auto-advancing task runner — [details](06-task-runner.md)
- [ ] 07: Interruption remedies — [details](07-interruption-remedies.md)
