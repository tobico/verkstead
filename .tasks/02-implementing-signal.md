# 02. The Implementing mid-turn signal

## What to build

Extend the launch-time detection from task 01 to the **Implementing** state,
where "cut off mid-turn" is not unconditional the way a grilling's is: a run
stops between tasks as routinely as mid-task, and only the mid-task one should be
carried on.

The signal is the task's **own state**, read the way `runner::backlog_again`
already reads it — the current `.tasks/` checkbox and whether the branch holds
that task's commit yet. A session cut off while its task was still being worked
(uncommitted, box unticked) is mid-turn and is carried on. A session that
committed its task and the run stopped cleanly between tasks is **not** mid-turn:
the next task is a fresh session, so it stays Verkstead's own Resume.

Fold this into the detection's decision rather than adding a parallel one: the
state tells it how to read "mid-turn", grilling being the unconditional case and
Implementing the task-state case.

## Acceptance criteria

- [ ] An implementing session cut off mid-task (its task uncommitted) is carried on with the harness's own resume.
- [ ] A session cut off cleanly between tasks (its task committed) stays Verkstead's own Resume, which launches a fresh session for the next task.
- [ ] Both are covered by tests reading the same `.tasks/` and branch state the runner reads.
