# Grilling

A grilling session runs entirely from the GUI. Pressing *start grilling* on a
Conversation creates its branch and worktree, launches the Conversation's
grilling Profile in a bwrap sandbox, streams the session's transcript into the
Timeline, and puts the agent's Question Sets to the human in the workbench and
on the phone alike. Blocking asks only — nothing after grilling exists yet.

This is MVP stage 02. Stage 01 left a Conversation that can be drafted but
never runs: the `Lifecycle` ladder already names `Grilling`, and the `Event`
enum holds nothing but the `Brief`. This stage is what fills both in, and it is
the first time Verkstead executes anything.

Roadmap stage: [02: Grilling](docs/roadmaps/mvp/02-grilling.md)

## Tasks

- [ ] 01: Worktree per Conversation — [details](01-worktree-per-conversation.md)
- [ ] 02: The sandbox surface — [details](02-sandbox-surface.md)
- [ ] 03: A sandbox under the unit — [details](03-sandbox-under-the-unit.md)
- [ ] 04: Grilling session with a captured transcript — [details](04-grilling-session.md)
- [ ] 05: Bundled skills — [details](05-bundled-skills.md)
- [ ] 06: Question Sets in the Timeline — [details](06-question-sets-in-the-timeline.md)
- [ ] 07: Retire the pending and archive namespaces — [details](07-retire-pending-and-archive.md)
