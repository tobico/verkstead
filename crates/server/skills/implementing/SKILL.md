---
name: implementing
description: Build work that has already been settled. Use when a Conversation's direction is to implement inline — primed with a Brief and the handoff document the grilling wrote, or with the Brief alone where there was no grilling.
---

Build the work described in the prompt: the Brief it started from, and — where
there was a grilling — the handoff document that session wrote on its way out.

The handoff is the agreement. Everything in it was settled with the human over a
whole grilling, so it is the design you are implementing rather than a first
draft of one — read it as decided. Where it is silent, decide as the codebase
would: match what is around the change, and prefer the smallest thing that does
the job.

**A Conversation can be started with no grilling at all**, and one that was says
so under its Brief. There is no handoff then because there was no interview to
write one: nothing is missing, and the Brief is the whole of the agreement.
Which makes it thinner than a handoff by design, so what it leaves genuinely
open is put to the human as a blocking ask rather than guessed at — a session
that guesses builds the wrong thing quietly, and one that asks reaches them on
their phone. That instruction is the prompt's own, under the Brief. Everything
else here is the same either way: the committing, the pull request and the
finish below do not care how the work came to be settled.

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
- **Leave nothing for a later session.** There is no step after this one: the
  branch is yours to carry all the way to the pull request, which is what the
  last section here is.

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

## Work the way the repository does

Read what the repository says about itself before writing code — its
`CLAUDE.md`, its `docs/`, the tests around what you are changing — and follow
it. Run its tests, and fix what you break: a green branch is part of the work
rather than a bonus on top of it.

## Then get the branch reviewed, the way this repository does it

The work is committed, so the last of it is getting it looked at. Read the
repository's `docs/agents/git-workflow.md` — its `## Review process`, and the
`### Finish sequence` inside it — and follow that sequence step by step. It is
the repository's process rather than Verkstead's, so what is written there is
what to do, whatever another project's habits would suggest.

Two shapes, and which one this is, is a fact about the branch rather than a
choice:

- an **unstacked** branch — the ordinary case — is pushed, and then opened as a
  **draft** pull request titled for the work, with a summary of what you built
  as its body;
- a **stacked** branch, one made with `gh stack init` / `gh stack add`, goes
  through `gh stack submit --auto` instead, after which this branch's own pull
  request has its title and body corrected. Leave the stack's other pull
  requests alone: they belong to finished work.

Work out which of the two this branch is before running either, and follow what
the repository's own sequence says about it — `gh stack view` naming this branch
is what says it is in a stack, and an error or a stack without it says it is
not. A repository whose file says nothing about finishing, or has no such file:
push the branch and open a draft pull request titled for the work —

    git push -u origin HEAD
    gh pr create --draft --title '<what this built>' --body '<what you built>'

**Nothing waits on approval here either.** No gate, no confirmation and nobody
at this terminal: the pull request opens unasked, and it opens as a *draft*
because merging is the human's act and nothing here is allowed to look like it
was theirs. Then stop — that is also what ends this session: Verkstead waits for
you to go quiet, finds the pull request, and takes the Conversation on to
wrapping it up.

### And every companion repository you committed in

This Conversation may be working alongside other repositories — the prompt
lists them under *Companion repositories*, each with where it is and what it is
holding. Every one of them you have committed in goes the same way, and nothing
you did above carried it anywhere: `cd` into that companion's worktree, read
*that* repository's own `docs/agents/git-workflow.md`, and follow its finish
sequence there. Its process rather than this one's, on its own branch, ending
in a pull request of its own.

Only the ones holding commits. A companion you read from and never committed in
needs nothing at all, and a read-only one could hold nothing to begin with —
`git log --oneline <base>..HEAD` in its worktree, against the commit its branch
was cut from, is what says which is which.

Verkstead asks GitHub about each of them once you have gone quiet, so a
companion holding commits and no pull request stops the run rather than being
carried on past.

### A session that finds the work already done

You may be the second session on this branch — the first one built the work and
went before it got this far. Nothing to build is not nothing to do: read what is
on the branch against the handoff, finish anything it left short, and then carry
it to the pull request exactly as above. A session that ended here saying there
was nothing for it to do would leave the branch where the one before it did,
which is the one ending this run cannot recover from by itself.

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
