---
name: breaking-down
description: Break settled work into a sequential .tasks/ backlog. Use when a Conversation's direction is to build from a task list — read on from the grilling that settled it.
---

Break the settled work into sequential task files under `.tasks/`, and commit
them. What you are deciding here is the *shape of the backlog*: how the work
splits, in what order, and where each slice stops.

Tasks are **always sequential** — no parallel work, no "blocked by". The order
is the dependency. Each task is worked later by a session of its own, with none
of this one's context, so a task has to carry everything its slice needs.

You start in a worktree of the repository, on a branch of its own. The branch is
already made and this is already the feature: there is nothing to create, nothing
to switch to, and no other plan in flight to check for.

## Where the agreement is

**You are the grilling session, reading on.** The human picked `task-list` on
your closing Set, and the agreement is this conversation — you are the one who
settled it. Trust your own context over any summary of it, and read what they
wrote beside the pick as part of what the backlog has to answer to. Step 1 is
largely behind you: you have been reading this codebase all along, so sweep for
what you have not looked at rather than starting over.

There is no handoff document, and there is nothing else to read the agreement
off: a task list writes no handoff, because the backlog *is* what the grilling
settled. The direction is settled and the breakdown is what is left.

## 1. Ground the plan in the code

Read the codebase before drafting anything. Task titles and descriptions should
use the project's own vocabulary — its `CLAUDE.md` or `AGENTS.md`, its
`CONTEXT.md` glossary if it has one, the ADRs under `docs/adr/` covering what
you are touching — and the breakdown should match how the code is actually
laid out rather than how the plan imagined it.

Where the code has drifted from what the grilling assumed, that is worth asking
about. Don't ask as you find it: finish grounding, then put the drift alongside
the breakdown in the one Set below.

## 2. Draft sequential tasks

Break the work into **sequential tracer-bullet tasks**. Each is a thin vertical
slice cutting through every integration layer end to end, not a horizontal slice
of one layer.

- Ordered — later tasks may depend on earlier ones.
- Small enough to fit comfortably in one focused context window.
- Each delivers demonstrable end-to-end behaviour on its own.
- Prefer many thin slices over few thick ones.

## 3. Put the breakdown to the human

The breakdown is a decision they own, so it goes to them as an ordinary Question
Set — the same blocking ask a grilling uses, and the way described under *How
the questions reach them* below.

Give the whole breakdown in the Preface, numbered, and for each task a title, a
one-line description, and two or three acceptance criteria. That is the context
the questions are read against. Then ask about it: whether the granularity is
right, whether anything should be merged, split or reordered, whether anything is
missing — plus whatever drift step 1 turned up.

Iterate until they approve the breakdown. Every round is an ordinary Set and
carries no `proposal` block: the direction is settled, so there is no closing
move left to make and nothing you send ends anything. What ends this session is
the plan commit below.

## 4. Write the task files

Pick a short kebab-case feature name — `credential-storage`, `add-model-wizard`
— for `TODO.md`'s heading. Do not rename the branch; it is the Conversation's and
it is already made.

Create `.tasks/` and write `TODO.md` plus one `NN-<slug>.md` per task, with
zero-padded numbers.

<todo-template>
# <Feature name>

<1-2 paragraph description of what is being built and why.>

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

## 5. Commit the plan

**Nothing waits on approval.** There is no gate, no confirmation and nobody at
this terminal to ask for one — the human approved the breakdown in step 3, and
the commit is how it gets written down.

    git add .tasks/
    git commit -m "chore: plan <feature-name> tasks"

If the planning turned up changes to `CONTEXT.md`, the ADRs under `docs/adr/`, or
other project documentation, include them in that commit: they belong on the
branch beside the plan that motivated them.

Then stop. **Do not start on task 01**, and do not say anything about clearing a
context — Verkstead reads `.tasks/` back off the branch and runs a session of its
own per task. That commit is also what ends this session: Verkstead sees the
backlog land, waits for you to go quiet, and takes it from there.

## How the questions reach them

Every question goes as a Question Set through the `verkstead` CLI, and nothing
else reaches anybody. There is no human at this terminal: the session runs on a
machine of its own and they answer on a phone, so a question printed here is one
nobody will ever see.

- **Read `verkstead guide` before the first ask.** It is everything the binary
  knows about asking well — how a Set is labelled, how much belongs in one, the
  shape it goes over the wire in, and how to run an ask that blocks for hours —
  and it ships inside the binary, so nothing else has to be found.
- **Put every round through `verkstead ask`.** It blocks until the answers come
  back, which may be hours. Idling is this working rather than this failing, so
  do only work the answers cannot invalidate while you wait.
- **Never answer on their behalf.** If the ask itself fails — the server
  unreachable, any non-zero exit that is not a refused Set — say so and stop.
  Approving your own breakdown decides in their place the very thing worth
  asking about.
