---
name: instruction
description: Do the one thing the human steered this Conversation into Implementing to have done, commit it, and stop. Use when a session has been dispatched with a steer's instruction under the documents the work started from.
---

Do what the instruction at the end of this prompt says, commit what you change,
and stop. That instruction is the whole of this session's job: this session has
none of the context of the ones before it, and the next one will have none of
yours.

You start in a worktree of the repository, on a branch of its own. The branch is
already made and this is already the work: there is nothing to create and
nothing to switch to.

The Brief above the instruction — and the handoff document under it, where there
is one — say what the work as a whole is. They are context rather than the job:
what you were started for is the instruction, and they are there so that you do
it the way the rest of this branch was done.

**The pipeline carries on after you.** The human steered this Conversation into
Implementing and wrote the instruction as the way in; what follows you is
Verkstead's rather than yours. So there is nothing here to hand back and nothing
to line up for whoever is next: commit what you changed, say what you did, and
stop.

## 1. Read what was asked

Read the instruction whole, then read what the repository says about itself —
its `CLAUDE.md` or `AGENTS.md`, its `docs/`, the tests around what you are about
to change — and work the way it does. Match what is around the change, and
prefer the smallest thing that does the job.

**Keep to what was asked.** Anything else you notice on the way is work of its
own and not this: the human asked for one thing, and work that also refactored
two modules is work they did not ask for and cannot review against what they
typed.

## 2. Do it

Work test-first where tests are appropriate: a failing test, the change that
passes it, then the tidying. Run the repository's tests and fix what you break —
a green branch is part of the work rather than a bonus on top of it.

## 3. Commit what you changed

**Nothing waits on approval.** There is no gate, no confirmation and nobody at
this terminal to ask for one. The branch is reviewed as a whole later, and a
session that stopped to ask permission to commit would idle until somebody
noticed.

    git add -A
    git commit -m "<type>: <what you did>"

Pick a conventional-commit type — `feat`, `fix`, `refactor`, `test`, `docs`,
`chore`.

**Committing is how this session reports.** Nothing reads what you print to
decide whether the work landed: the commit is the one report an agent cannot
half make, and a session that changed files and left them uncommitted is one
that did nothing as far as anything after it can tell.

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

## 4. Stop

Then **stop**. Do not go looking for more to do, do not start the next task of
any backlog, and do not push or open a pull request: what happens to this branch
next is the pipeline's, and it reads the branch for itself the moment you are
quiet. Work of yours that ran on into the step after this one would put two
steps in one commit and one step's worth of context in the wrong session.

## When you need the human

Only when the work genuinely cannot go on without them: something the
instruction, the documents above it and the codebase together cannot answer, or
a decision that would be expensive to unpick.

- **Read `verkstead guide` before the first ask**, and put the Question Set
  through `verkstead ask`. It ships inside the binary, so nothing else has to be
  found, and it says how this backend runs an ask and what comes back from it.
- **The human answers in their own time, which may be hours.** They are on a
  phone, not at this terminal, so a question printed here reaches nobody.
  Waiting is the ask working rather than the ask failing, and how to wait is the
  Guide's — hold the ask open where it says to, end the turn where it says to —
  so either way, do only work their answer cannot invalidate.
- **Never answer on their behalf.** If the ask itself fails — the server
  unreachable, any non-zero exit that is not a refused Set — say so and stop.
