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

Read `.tasks/TODO.md`. The next step is the **lowest-numbered entry whose box is
not ticked** — the box is what says a task is done, so an unticked one is what
is still to do. The `NN-<slug>.md` file it links to is that task's document, and
every task's document stays in `.tasks/` from the moment it is written until the
feature is finished: what is there says nothing about what is done.

**If every entry is ticked, the feature is done**: go to *Finishing the feature*
below instead.

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

Tick this task's entry in `TODO.md`, and commit that alongside the code. Leave
the task file where it is — the whole of `.tasks/` goes at the finish, rather
than a file at a time:

    # edit .tasks/TODO.md: "- [ ] NN: ..." becomes "- [x] NN: ..."
    git add -A
    git commit -m "<type>: <what the task delivered>"

Pick a conventional-commit type — `feat`, `fix`, `refactor`, `test`, `docs`,
`chore`. The commit is how the task is reported done: the box ticked *and*
committed is what says this step is over, so leave nothing uncommitted.

### What the message body says

A commit that delivers work — code, tests or documentation the work asked for —
carries a summary as its message body. That body is what the workbench shows
beside the diff when the human reviews this branch later, so it is written for
the reviewer who reads it before reading the patch. Pure bookkeeping carries
none: a plan or backlog commit, a roadmap commit, the finish commit, an ADR
recorded along the way. A commit still counts as delivering work when the list's
tick rides along with the code.

- **The prose first** — what you built and how it hangs together.
- **The diagram after it**, whenever the diff is more than three changed lines.
  The words are what the reviewer reads and the picture is what they check them
  against, so it sits under the prose and above the trailers. Diagram the
  delta rather than the system: the parts this change touches and the
  relationships between them, and nothing else. Tag each node `new`, `modified`
  or `removed` — the workbench colours those from the diff's own added and
  removed shades, so the picture and the patch read as one account of the
  change. Around ten nodes, so that it reads on a phone.

Trailers go at the end as usual; the workbench takes them off what it shows.

    feat: share the rate limiter's count between instances

    The counter moves out of the process, so every instance counts against the
    same window, and the in-process throttle it replaces goes away.

    ```mermaid
    flowchart LR
      api[POST /v1/messages] --> limiter[Rate limiter]
      limiter --> counter[(Redis counter)]
      api --> throttle[In-process throttle]

      class limiter,counter new
      class api modified
      class throttle removed
    ```

Then **stop**. Do not start the next task, and do not say anything about clearing
a context — Verkstead runs a fresh session of its own for it, and starting on it
here would put two tasks in one commit and one task's worth of context in the
wrong session.

## Finishing the feature

Reached when every entry in `TODO.md` is ticked: every task has been worked.
There is no gate in front of this either — nothing is left to do, which is the
only thing finishing was ever waiting on.

Take the backlog away — the list and the task files together — and commit that:

    git rm -r .tasks/
    git add -A
    git commit -m "chore: finish <feature-name>"

The feature name is `TODO.md`'s own heading.

### Then get the branch reviewed, the way this repository does it

Read the repository's `docs/agents/git-workflow.md` — its `## Review process`,
and the `### Finish sequence` inside it — and follow that sequence step by step.
It is the repository's process rather than Verkstead's, so what is written there
is what to do, whatever another project's habits would suggest.

Two shapes, and which one this is, is a fact about the branch rather than a
choice:

- an **unstacked** branch — the ordinary case — is pushed, and then opened as a
  **draft** pull request titled for the feature, with a summary of the completed
  tasks as its body;
- a **stacked** branch, one made with `gh stack init` / `gh stack add`, goes
  through `gh stack submit --auto` instead, after which this branch's own pull
  request has its title and body corrected. Leave the stack's other pull
  requests alone: they belong to finished work.

Work out which of the two this branch is before running either, and follow what
the repository's own sequence says about it — `gh stack view` naming this branch
is what says it is in a stack, and an error or a stack without it says it is not. A repository whose file says
nothing about finishing, or has no such file: push the branch and open a draft
pull request titled for the feature —

    git push -u origin HEAD
    gh pr create --draft --title '<feature name>' --body '<the tasks this delivered>'

**Nothing waits on approval here either.** No gate, no confirmation and nobody
at this terminal: the pull request opens unasked, and it opens as a *draft*
because merging is the human's act and nothing here is allowed to look like it
was theirs. Then stop.

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
