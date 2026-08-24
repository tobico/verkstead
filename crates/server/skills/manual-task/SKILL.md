---
name: manual-task
description: Do the one thing the human typed at the end of a Conversation's timeline, and commit it. Use when a session has been dispatched with a manual task's instruction as the whole of its prompt.
---

Do what the instruction at the end of this prompt says, and commit what you
change. That instruction is the whole of the job: this session has none of the
context of the ones before it, and the next one will have none of yours.

You start in a worktree of the repository, on a branch of its own. The branch is
already made and this is already the work: there is nothing to create and
nothing to switch to.

This is a **manual task** — the human asked for it by hand, outside whatever
else the Conversation is doing. It is not a step of a backlog, not a piece of
review feedback, and not a stage of anything. Nothing was written down about it
anywhere: the instruction you were given is all there is, and it is enough.

## 1. Read what was asked

Read the instruction whole, then read what the repository says about itself —
its `CLAUDE.md`, its `docs/`, the tests around what you are about to change —
and work the way it does. Match what is around the change, and prefer the
smallest thing that does the job.

**Keep to what was asked.** Anything else you notice on the way is another
manual task and not this one: the human asked for one thing, and work that also
refactored two modules is work they did not ask for and cannot review against
what they typed.

## 2. Do it

Work test-first where tests are appropriate: a failing test, the change that
passes it, then the tidying. Run the repository's tests and fix what you break —
a green branch is part of the work rather than a bonus on top of it.

Some instructions are not code at all — rebase this, push that, read something
and say what you found. Do those as they are asked, and do not turn them into
code changes.

## 3. Commit what you changed

**Nothing waits on approval.** There is no gate, no confirmation and nobody at
this terminal to ask for one. The branch is reviewed as a whole later, and a
session that stopped to ask permission to commit would idle until somebody
noticed.

    git add -A
    git commit -m "<type>: <what you did>"

Pick a conventional-commit type — `feat`, `fix`, `refactor`, `test`, `docs`,
`chore`.

### What the message body says

A commit that delivers work — code, tests or documentation the work asked for —
carries a summary as its message body. That body is what the workbench shows
beside the diff when the human reviews this branch later, so it is written for
the reviewer who reads it before reading the patch. Pure bookkeeping carries
none: a plan or backlog commit, a roadmap commit, the finish commit, an ADR
recorded along the way. A commit still counts as delivering work when a task
file's deletion rides along with the code.

- **The diagram first**, whenever the diff is more than three changed lines.
  The glance comes before the reading, so it goes above the prose. Diagram the
  delta rather than the system: the parts this change touches and the
  relationships between them, and nothing else. Tag each node `new`, `modified`
  or `removed` — the workbench colours those from the diff's own added and
  removed shades, so the picture and the patch read as one account of the
  change. Around ten nodes, so that it reads on a phone.
- **The prose after it** — what you built and how it hangs together.

Trailers go at the end as usual; the workbench takes them off what it shows.

    feat: share the rate limiter's count between instances

    ```mermaid
    flowchart LR
      api[POST /v1/messages] --> limiter[Rate limiter]
      limiter --> counter[(Redis counter)]
      api --> throttle[In-process throttle]

      class limiter,counter new
      class api modified
      class throttle removed
    ```

    The counter moves out of the process, so every instance counts against the
    same window, and the in-process throttle it replaces goes away.

If the instruction asked for nothing that changes files, commit nothing. That is
a manual task done, not a manual task failed — say what you found as the last
thing you print, because that is what the human sees on the timeline.

Then **stop**. Do not go looking for more to do, and do not open a pull request:
what happens to this branch next is the human's to decide.

## Asking, if you need to

You **may** put a Question Set to the human, and nothing here says you have to.
A manual task is usually one instruction that already says what it means, and a
question asked about work you already understand is a session idling for hours
over nothing.

Ask when the instruction genuinely cannot be carried out as it stands: it means
two different things and the difference is expensive, or it asks for something
the repository says not to do.

- **Read `verkstead guide` before the first ask**, and put the Question Set
  through `verkstead ask`. It ships inside the binary, so nothing else has to be
  found.
- **It blocks until they answer, which may be hours.** They are on a phone, not
  at this terminal, so a question printed here reaches nobody. Run the ask as a
  background command and do only work their answer cannot invalidate while you
  wait.
- **Never answer on their behalf.** If the ask itself fails — the server
  unreachable, any non-zero exit that is not a refused Set — say so and stop.
