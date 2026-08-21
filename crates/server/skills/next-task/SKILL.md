---
name: next-task
description: Work one task of a .tasks/ backlog, or finish the feature once none are left. Use when a Conversation is working through a task list and this session has been launched for the next step.
---

Work **one** task of the `.tasks/` backlog in this worktree, commit it, and stop.
One task per session is the whole point: this session has none of the context of
the ones before it, and the next one will have none of yours.

You start in a worktree of the repository, on a branch of its own. The branch is
already made and this is already the feature: there is nothing to create and
nothing to switch to. The prompt carries the Brief the work started from and the
handoff document the grilling settled — they say what the backlog is a breakdown
*of*, and the task file says where this session stops.

## 1. Find the step

Look in `.tasks/`. The next step is the **lowest-numbered `NN-<slug>.md` file
that is still there** — `TODO.md` is the list rather than a task, and never
matches. Finishing a task is what deletes its file, so what is left is what is
still to do.

**If no numbered task files remain, the feature is done**: go to *Finishing the
feature* below instead.

## 2. Read the task

Read the whole task file and summarise its goal to yourself before writing any
code. Its acceptance criteria are what *done* means — nothing above them and
nothing beside them.

Then read what the repository says about itself — its `CLAUDE.md`, its `docs/`,
the tests around what you are changing — and work the way it does. Match what is
around the change, and prefer the smallest thing that does the job.

## 3. Build it

Work test-first where tests are appropriate: a failing test, the change that
passes it, then the tidying. Run the repository's tests and fix what you break —
a green branch is part of the work rather than a bonus on top of it.

Keep the scope to this task's acceptance criteria. Work the next task's slice
into this one and the session after yours will be reading a task file for work
that has already been done.

## 4. Commit the task

**Nothing waits on approval.** There is no gate, no confirmation and nobody at
this terminal to ask for one. The branch is reviewed as a whole later, and a
session that stopped to ask permission to commit would idle until somebody
noticed.

Delete the task file and tick its entry in `TODO.md`, and commit both alongside
the code:

    rm .tasks/NN-<slug>.md
    # edit .tasks/TODO.md: "- [ ] NN: ..." becomes "- [x] NN: ..."
    git add -A
    git commit -m "<type>: <what the task delivered>"

Pick a conventional-commit type — `feat`, `fix`, `refactor`, `test`, `docs`,
`chore`. The commit is how the task is reported done: the file being gone *and*
committed is what says this step is over, so leave nothing uncommitted.

Then **stop**. Do not start the next task, and do not say anything about clearing
a context — Verkstead runs a fresh session of its own for it, and starting on it
here would put two tasks in one commit and one task's worth of context in the
wrong session.

## Finishing the feature

Reached when `.tasks/` holds nothing but `TODO.md`: every task has been worked.
There is no gate in front of this either — the backlog is empty, which is the
only thing finishing was ever waiting on.

Take the list away and commit that:

    git rm .tasks/TODO.md
    git add -A
    git commit -m "chore: finish <feature-name>"

The feature name is `TODO.md`'s own heading. **Do not push and do not open a
pull request** — getting the branch reviewed is a step of its own that Verkstead
runs after this one. Then stop.

## When you need the human

Only when the work genuinely cannot go on without them: something the task file,
the two documents and the codebase together cannot answer, or a decision that
would be expensive to unpick.

- **Read `verkstead guide` before the first ask**, and put the Question Set
  through `verkstead ask`. It ships inside the binary, so nothing else has to be
  found.
- **It blocks until they answer, which may be hours.** They are on a phone, not
  at this terminal, so a question printed here reaches nobody. Run the ask as a
  background command and do only work their answer cannot invalidate while you
  wait.
- **Never answer on their behalf.** If the ask itself fails — the server
  unreachable, any non-zero exit that is not a refused Set — say so and stop.
