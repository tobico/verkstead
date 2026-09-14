# clear-stale-task-list

A branch cut from a base that already carries a `.tasks/` list is read as having
planned the moment its session is launched: the watcher that ends a planning
session checks only that `.tasks/TODO.md` exists and is committed, never whether
this branch wrote it. A roadmap continued from a stage that stopped part way had
its next planning session ended before it had asked anything.

Verkstead now clears the inherited list itself, as a commit of its own on the
fresh branch, authored as the configured git author, and refuses to start work
where no author is configured. Only the Conversation's own worktree is touched,
and only on a cut for new work: a grilled start, an ungrilled build, Continue a
roadmap, and a stage started by its predecessor settling. A take-up and a steer's
re-checkout are left alone. The reading that says whether a branch has planned
yet learns that a deletion is not a writing, so neither the clearing commit nor a
planning session that died before committing can be mistaken for a plan.

## Tasks

- [x] 01: A deletion under `.tasks/` is not a plan written — [details](01-a-deletion-is-not-a-plan.md)
- [x] 02: A start a human presses clears the inherited list — [details](02-a-pressed-start-clears-the-list.md)
- [x] 03: An unattended stage start clears the same way, and the decision is written down — [details](03-a-stage-start-clears-the-same-way.md)
