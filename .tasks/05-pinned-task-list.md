# 05. The pinned task-list Event

## What to build

The backlog task 04 wrote becomes visible: Verkstead parses `.tasks/` out of the
Worktree and shows it as the task-list Event, pinned on the Timeline, listing
each task and whether it is done.

**Pinning does not exist yet.** The Timeline holds Moved, Brief, agent output
and Question Set events, all of which scroll past in order, and a pinned Event
is a different thing — it stays in view because it is the current state of
something rather than a record of a moment. The domain model fixes the shape:
task lists, stage lists and PRs are pinned, it is a fixed set, and there is no
manual pin or unpin. So the concept gets built here, with the task list as its
first user.

Parsing follows what to-tasks writes and roadrunner already reads: `TODO.md`'s
checkbox list, and `NN-<slug>.md` task files matched by their leading number.
The parse is of the Worktree as it stands, so the Event tracks the backlog as it
changes rather than as it was first written — which is what makes it useful in
task 06, when tasks start completing on their own.

## Acceptance criteria

- [ ] Events can be pinned, as a fixed set with no manual pin or unpin
- [ ] A pinned Event stays in view rather than scrolling past with the record
- [ ] `.tasks/TODO.md` and the numbered task files parse into the task-list
      Event
- [ ] The Event shows each task and its done state
- [ ] It reflects the Worktree as it stands, updating when `.tasks/` changes
- [ ] A Conversation with no `.tasks/` shows no task-list Event
