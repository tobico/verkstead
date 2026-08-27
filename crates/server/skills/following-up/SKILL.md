---
name: following-up
description: Follow up on work that is already on a pull request — answer what the human asked, do what they requested, and keep the conversation going in rounds of Question Sets. Use when a session has been dispatched with a follow-up brief under the documents the work started from.
---

Do what the brief at the end of this prompt asks, and keep talking to the human
about it. This is a conversation rather than a single step: they wrote the brief
to you, and each round of it goes back to them as a Question Set.

You start in a worktree of the repository, on the branch the work is on. The
branch is already pushed and it already has a pull request open: there is
nothing to create, nothing to switch to and nothing to open.

The Brief above the follow-up brief — and the handoff document under it, where
there is one — say what the work as a whole is. They are context rather than the
job: what you were started for is the follow-up brief, and they are there so
that you answer and work the way the rest of this branch was done.

## 1. Read what was asked

**The brief is written to this session**, in the human's own words and aimed
here. Act on what it plainly asks: answer the questions in it, do the work it
requests. It is not a remark somebody left in passing and it is not a proposal
to be put back to them — they have already decided that this is what they want,
which is why they typed it into a session rather than onto the pull request.

So there is nothing to propose first, and no Set to write before you start.
**Ask ahead of doing only what is genuinely ambiguous, destructive, or beyond
what the brief said**: a choice the brief does not settle and the code cannot,
something that would be expensive to unpick, work nobody asked for. Everything
else you simply do.

Then go and look. An answer written from memory of the diff is a guess:

    gh pr diff

Read the code each part of the brief is about, and read what the repository
says about itself where the answer turns on a convention — its `CLAUDE.md` or
`AGENTS.md`, the docs it keeps for agents, what the neighbouring code actually
does.

## 2. Do the work, and push it as it lands

Work test-first where tests are appropriate: a failing test, the change that
passes it, then the tidying. Run the repository's tests and fix what you break —
a green branch is part of the work rather than a bonus on top of it.

**Keep to what was asked.** Anything else you notice on the way is work of its
own and not this: they asked for a thing, and work that also refactored two
modules is work they cannot review against what they typed.

**Nothing waits on approval.** There is no gate, no confirmation and nobody at
this terminal to ask for one: the brief was the approval, and what you go on to
ask about goes to their phone rather than to this screen.

One commit per thing you were asked for, so each reads against the part of the
brief that asked for it:

    git add -A
    git commit -m "<type>: <what you did>"

Pick a conventional-commit type — `feat`, `fix`, `refactor`, `test`, `docs`,
`chore`.

Then push, **before you ask them anything**:

    git push

Push, unlike most sessions here: this branch is already on a pull request, so
the pull request always shows what has been done, and the checks on what you
pushed run while they are reading and composing. Work that stayed local while
they answered is work they were answering blind about.

Do not open a pull request, do not touch any other branch, and do not merge
anything. The pull request exists, and merging is the human's act.

### What the message body says

A commit that delivers work — code, tests or documentation the work asked for —
carries a summary as its message body. That body is what the workbench shows
beside the diff when the human reviews this branch later, so it is written for
the reviewer who reads it before reading the patch. Pure bookkeeping carries
none: a plan or backlog commit, a roadmap commit, the finish commit, an ADR
recorded along the way. A commit still counts as delivering work when a task
file's deletion rides along with the code.

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

## 3. Put the round to the human as one Question Set

**The human sees nothing that does not arrive as a Set.** They are on a phone
rather than at this terminal, so what you print here reaches nobody: everything
you want them to read — the answers to what they asked, what you did about it,
what you need decided next — goes into the Set.

What you send is an ordinary Question Set, the same shape as every other ask.
Nothing about this session makes it a special one.

- **The answers lead.** What they asked, answered, goes in the `preface`, with
  what you changed and pushed. That is the half of the round they are owed, and
  the Set is how it reaches them.
- **A decision you need is a Question**, with its Options in your own words and
  the one you would take recommended. Each credible way to do it is an Option of
  its own; offer alternatives only where more than one credible way exists.
- **The `postscript` is an ordinary postscript** — the open-ended invitation,
  above the comment box that shares its section, and nothing that obliges a
  reply. A decision, however small, is a Question instead.
- **One Set for the round**, rather than one per thing you did.
- **Read `verkstead guide` before you write it** — how a Set is labelled, how
  much belongs in one, and the shape it goes over the wire in. It ships inside
  the binary, so nothing else has to be found.

Then put it through `verkstead ask`, **as a background command**: it blocks
until they answer, and that may be hours.

**The answers are yours to wait for.** Nobody else is dispatched to act on what
they say, so there is nothing to do while you wait: do not start on what you
have only proposed, and do not take your own recommendations.

If the ask itself fails — the server unreachable, any non-zero exit that is not
a refused Set — say so and stop. Never answer on their behalf.

## 4. Read the Response, and go round again

The Response is the whole of what you act on. Read all of it, the `comment` on
the Set included, before you touch anything: it is about the Set as a whole and
may reframe the answers above it, and it is where a new thing to do usually
arrives.

- **What they picked is what you do**, the way they picked it rather than the
  way you would have.
- **What they wrote beside an answer is part of the instruction**, and it is
  the reason their words come back to you at all.
- **A fresh ask in the Response is the next round's brief**, read the way the
  first one was: theirs, aimed here, and done rather than proposed back.
- **Unanswered is not a yes.** Leave it.

Then round again from step 2: do it, commit it, push it, and put the next Set.
A follow-up is as many rounds as they want it to be.

## 5. Finishing your turn

When a Response leaves you with nothing to do and nothing to ask, say what you
did and **finish your turn**. That is all: no closing line to anybody, no
summing-up of the follow-up, nothing to hand on.

**What becomes of this Conversation is not yours.** Do not mark the pull request
ready, do not merge it, do not move the work anywhere, and do not decide that
the follow-up is over — Verkstead reads how this session stands rather than
anything you say about it, and it knows what comes next.
