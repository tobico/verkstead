# 04. Agent profiles

## What to build

The last record a grilling session needs: which account and model it runs
under.

**A profile is minimal** — a name, a claude home directory and config file
pair, a default model, and an agent-type discriminator. The pair is what gets
bind-mounted over `~/.claude` and `~/.claude.json` inside the sandbox next
stage, which is the account-separation trick `tobico-scripts/bin/work-sandbox`
uses today. The discriminator exists so another agent backend can slot in
later; claude is the only type now, and the point is that the column is there
rather than that anything branches on it.

Profiles are managed from the workbench: created, edited and deleted. Validate
the dir and config pair at save time — both must exist, and both must sit
inside the watched paths, since the same boundary governs every filesystem
operation. A profile whose pair has since disappeared should read as broken
rather than silently fail when a session is later launched against it.

**A conversation fixes two profiles before grilling starts**: one for grilling,
one for implementation. They are separate choices because they are genuinely
different accounts and models — grill on fable, implement on opus, today — and
because the implementation session cannot simply continue the grilling one.
Both are chosen on the conversation, and both are required before the next
stage will let grilling begin.

This closes the stage: a conversation now carries a repo, a brief, a branch
name, a base-commit rule and two profiles — everything *start grilling* will
need.

## Acceptance criteria

- [ ] A profile is created, edited and deleted from the workbench, and persists across a restart
- [ ] A profile whose claude dir or config file does not exist is refused at save time, with the reason shown
- [ ] A profile whose pair sits outside the watched paths is refused, by the server
- [ ] A profile whose pair has disappeared since it was saved reads as broken in the UI
- [ ] A conversation selects a grilling profile and an implementation profile independently, and both persist
- [ ] A conversation missing either profile is identifiably not ready to grill
- [ ] The phone's answering flow still works unchanged
