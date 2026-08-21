---
name: next-stage
description: Start a roadmap stage: re-ground its brief against the code, quiz the human on the breakdown, and write the .tasks/ backlog. Use when a Conversation is a stage of a roadmap and this session has been launched to plan it.
---

Turn the stage brief in the prompt into a sequential `.tasks/` backlog in this
worktree, commit it, and stop.

The brief was written when the roadmap was staged, out of a whole grilling. It is
the agreement about *what this stage is for* — but its task chunking is
explicitly provisional, because the codebase has moved since it was written and
the stages before this one have landed. Re-grounding that chunking against the
code as it now is, is the whole reason this session exists rather than the
backlog being written when the roadmap was.

You start in a worktree of the repository, on a branch of its own. The branch is
already made and this is already the stage: there is nothing to create, nothing
to switch to, and no other plan in flight to check for.

## 1. Re-ground the brief in the code

Read the brief in full first, then read the code it is about.

- Work through the brief's **Re-verify at start** list item by item, against the
  actual codebase. That list is the previous session's own account of what it
  expected to go stale.
- Read the roadmap's `ROADMAP.md` beside the brief for the stages around this
  one, and whatever the brief's **Decisions in force** section references that
  you need — the design documents, the ADRs under `docs/adr/`, the project's
  `CLAUDE.md` and its `CONTEXT.md` glossary.
- Use the project's own vocabulary in everything you write, and match how the
  code is actually laid out rather than how the brief imagined it.

Where things have drifted, adjust the breakdown. Don't ask as you find it:
finish grounding, then put the drift alongside the breakdown in the one Set
below.

## 2. Draft sequential tasks

Take the brief's **Proposed tasks (provisional)** as the draft and correct it.
The rules are the backlog's rather than the roadmap's:

- **Sequential** — no parallel work, no "blocked by". The order is the
  dependency.
- Thin vertical tracer-bullet slices, each cutting through every integration
  layer end to end rather than through one layer.
- Small enough to fit comfortably in one focused context window: each is worked
  later by a session of its own, with none of this one's context, so a task has
  to carry everything its slice needs.
- Each delivers demonstrable end-to-end behaviour on its own.

Keep to this stage. Work that belongs to a later stage stays in that stage's
brief.

## 3. Put the breakdown to the human

The breakdown is a decision they own, so it goes to them as an ordinary Question
Set — the way described under *How the questions reach them* below. **This is
the only thing that stops the run**, and it stops it naturally: the ask blocks
until they answer, from wherever they are.

Give the whole breakdown in the Preface, numbered, and for each task a title, a
one-line description, and two or three acceptance criteria. Say which of it
came from the brief and which is your correction. That is the context the
questions are read against. Then ask about it: whether the granularity is right,
whether anything should be merged, split or reordered, whether anything is
missing — plus whatever drift step 1 turned up.

Iterate until they approve the breakdown. Every round is an ordinary Set:
nothing in this session ends anything.

## 4. Put the branch in the stack, if the prompt says it is stacked

The prompt says what this branch came off. Where it says the branch **stacks on**
a named predecessor, the branch has the predecessor's commits under it already
and what is left is registering it. Read the repository's
`docs/agents/git-workflow.md` — its `## Review process`, and the
`### Stacking roadmap stages` block inside it — and do what that block says
about adding a branch that already exists to a stack.

That block is the repository's own mechanism, so what is written there is what to
do. Do not invent one, do not rebase anything, and do not touch the predecessor's
branch: it is finished work waiting on a human to merge it. If registering the
stack fails, say so plainly and carry on with the rest — the plan matters more
than the bookkeeping, and a stack can be registered afterwards.

Where the prompt says the branch is off the default branch, there is nothing to
do here.

## 5. Write the task files

Create `.tasks/` and write `TODO.md` plus one `NN-<slug>.md` per task, with
zero-padded numbers. `TODO.md`'s heading is the stage's own title.

<todo-template>
# <Stage title>

<1-2 paragraph description of what this stage delivers and why, from the brief's
Goal.>

Roadmap stage: [NN: <title>](docs/roadmaps/<name>/NN-<slug>.md)

## Tasks

- [ ] 01: <title> — [details](01-<slug>.md)
- [ ] 02: <title> — [details](02-<slug>.md)
- ...
</todo-template>

<task-file-template>
# NN. <Task title>

## What to build

Concise description of this vertical slice. Describe end-to-end behaviour, not
layer-by-layer implementation.

Avoid specific file paths or code snippets — they go stale fast. Exception: if a
decision is encoded more precisely by a snippet than by prose (a state machine, a
schema, a type shape), inline the decision-rich parts.

## Acceptance criteria

- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3
</task-file-template>

The `Roadmap stage:` line is what says this backlog is a stage's rather than a
feature's, and finishing the feature reads it back to tick the stage off.

## 6. Bring the roadmap's own score up to date

`ROADMAP.md` keeps the score of the whole effort, and it is one step behind by
the time you are reading it. Two edits, both in the plan commit below:

- **Tick every stage above this one that is still annotated as in progress.**
  Their work is finished — a stage is only started once the one before it has
  settled, which is why this session is running at all. Change `- [ ]` to
  `- [x]` and drop the `*(in progress: …)*` annotation.
- **Annotate this stage as in progress**, with the branch you are on, after its
  link:

      - [ ] NN: <title> — [brief](NN-<slug>.md) *(in progress: `<branch>`)*

  Leave the box unticked: the stage is under way rather than done, and the
  session that starts the stage after this one is what ticks it.

Do not renumber, reorder or reword anything else in the file.

## 7. Commit the plan

**Nothing waits on approval.** There is no gate, no confirmation and nobody at
this terminal to ask for one — the human approved the breakdown in step 3, and
the commit is how it gets written down.

    git add -A
    git commit -m "chore: plan <stage-name> tasks"

The roadmap edits from step 6 ride in that commit: the score moves on the branch
that earned it.

If the re-grounding turned up changes to `CONTEXT.md`, the ADRs under
`docs/adr/`, or other project documentation, include them too — they belong on
the branch beside the plan that motivated them.

Then stop. **Do not start on task 01**, and do not say anything about clearing a
context — Verkstead reads `.tasks/` back off the branch and runs a session of its
own per task.

## How the questions reach them

Every question goes as a Question Set through the `verkstead` CLI, and nothing
else reaches anybody. There is no human at this terminal: the session runs on a
machine of its own and they answer on a phone, so a question printed here is one
nobody will ever see.

- **Read `verkstead guide` before the first ask.** It is everything the binary
  knows about asking well — how a Set is labelled, how much belongs in one, and
  the shape it goes over the wire in — and it ships inside the binary, so
  nothing else has to be found.
- **Put every round through `verkstead ask`.** It blocks until the answers come
  back, which may be hours. Idling is this working rather than this failing, so
  run it as a background command and do only work the answers cannot invalidate
  while you wait.
- **Never answer on their behalf.** If the ask itself fails — the server
  unreachable, any non-zero exit that is not a refused Set — say so and stop.
  Approving your own breakdown decides in their place the very thing worth
  asking about.
