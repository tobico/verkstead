---
name: implementing
description: Build work that has already been grilled and settled. Use when a Conversation's direction is to implement inline, and the session is primed with a Brief and the handoff document the grilling wrote.
---

Build the work described in the prompt: the Brief it started from, and the
handoff document the grilling session wrote on its way out.

The handoff is the agreement. Everything in it was settled with the human over a
whole grilling, so it is the design you are implementing rather than a first
draft of one — read it as decided. Where it is silent, decide as the codebase
would: match what is around the change, and prefer the smallest thing that does
the job.

You start in a worktree of the repository, on a branch of its own. Nothing else
of the machine is reachable, so there is nothing here to be careful of but the
work.

## Commit as you go

**Nothing waits on approval.** There is no gate, no confirmation and nobody at
this terminal to ask for one — the branch is reviewed as a whole later, and a
session that stopped to ask permission to commit would idle until somebody
noticed.

- **Commit each coherent piece as you finish it**, with a conventional-commit
  message saying what changed and why. Every commit lands on the Conversation's
  Timeline, where the human reads it.
- **Leave the work committed.** Uncommitted changes at the end of a session are
  work nobody can see and nothing can build on.
- **Do not push, and do not open a pull request.** Getting the branch reviewed is
  a step of its own that Verkstead runs after this one.

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

## Work the way the repository does

Read what the repository says about itself before writing code — its
`CLAUDE.md`, its `docs/`, the tests around what you are changing — and follow
it. Run its tests, and fix what you break: a green branch is part of the work
rather than a bonus on top of it.

## When you need the human

Only when the work genuinely cannot go on without them: something the handoff
does not cover and the codebase cannot answer, or a decision that would be
expensive to unpick.

- **Read `verkstead guide` before the first ask**, and put the Question Set
  through `verkstead ask`. It ships inside the binary, so nothing else has to be
  found.
- **It blocks until they answer, which may be hours.** They are on a phone, not
  at this terminal, so a question printed here reaches nobody. Run the ask as a
  background command and do only work their answer cannot invalidate while you
  wait.
- **Never answer on their behalf.** If the ask itself fails — the server
  unreachable, any non-zero exit that is not a refused Set — say so and stop.
