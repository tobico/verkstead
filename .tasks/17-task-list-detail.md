# 17. Task list detail page

## What to build

Make the task list card clickable, opening a detail view in the details pane
— the same pane structure a commit or an answer set uses. The view stacks
**every task document** as its own boxed markdown section (the answer set's
Preface treatment: heading, then the padded bordered block), in backlog
order, titled by task number and name, with the pane's jump navigation to hop
between them. Done tasks whose files are gone render as their entry with a
note that the document is finished and removed.

A new endpoint serves the pane: it reads each task document off the worktree
and renders its markdown server-side (sanitized, diagrams carved out) the way
the commit pane's endpoint does, returning the sections in order. Wire types
are exported to the generated TypeScript as the others are. Both copies of
the card (sticky and, after task 16, the record row) open it.

## Acceptance criteria

- [ ] Tapping a task list card opens the pane showing every remaining task
      document rendered as markdown in boxed sections with jump navigation
- [ ] The endpoint refuses cleanly when the worktree or backlog is gone
- [ ] Route tests cover the endpoint; web tests cover the pane
