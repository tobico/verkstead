---
name: responding
description: Answer a batch of comments left on a pull request: propose what to do about them as one Question Set, and land what the human accepts. Use when a session has been dispatched with fresh pull request comments as its feedback.
---

Answer what has just been said on one of this Conversation's pull requests.
**You propose, and then you fix what was agreed to.** Nothing anybody wrote is
acted on before the human has said so, and everything they say yes to is done
here rather than by somebody else afterwards.

The comments are at the end of your prompt, under **What has just been said on
the pull request**: the batch whole, in the order it was said in, and where each
of it was said. Every comment names the pull request it was left on, and one
left on a line of the diff carries its file and line beside it — both halves are
what it means.

**Work where the feedback says.** This Conversation may hold a pull request in
more than one repository — its own, and one per companion repository the work
committed in — so the comments name the repository, the pull request and the
worktree to work in. `cd` into that worktree first and do the whole job there:
`git` and `gh` both read their repository from wherever they are run, so a
`gh pr diff` from the wrong directory reads somebody else's pull request, and a
push from there puts your answer on somebody else's branch.

You start in a worktree of the Conversation's own repository. Whichever worktree
you end up working in, it is one branch and one pull request: already pushed and
already open, so there is nothing to create, nothing to switch to and nothing to
open.

## 1. Read what was said, and what it is about

Read the batch whole before you form an opinion about any part of it. It is one
person's thinking rather than a list — three replies in a minute are one point
being made — so the last of them often says what the first was getting at.

Then go and look. A comment names a symptom and the code says what the cause
is — in the worktree the comments named, `gh` reading its repository from
wherever it is run:

    gh pr diff

Read what each one is about and the code around it, and read what the
repository says about itself where the answer turns on a convention — its
`CLAUDE.md` or `AGENTS.md`, the docs it keeps for agents, what the neighbouring
code actually does.

The Brief and the handoff in your prompt say what the work was *for*. A comment
asking for something the work was never meant to do is still worth proposing,
and worth saying so in the same breath.

## 2. Work out what each one is asking for

Some ask for a change. Some are a question, which you can answer in the
Question's own text without changing anything. Some are somebody saying they
are happy, and inventing work out of agreement is not answering it.

**You are the only session that will act on these.** Nothing else is dispatched
about this batch, so a comment you leave out is a comment nobody answers.

What you must not do is act on one because it is the human talking. A comment is
still a proposal until they have said yes to *this* reading of it: the words on a
pull request are not the same thing as an instruction to a session, and the
answer to "this is the wrong way round" is a decision they have not made yet.

## 3. Change nothing yet

No edits, no commits, no pushes, no `gh` command that writes anything — not
until the human has answered. Doing what a comment asks before they have seen
what you took it to mean is deciding in their place, and the whole point of the
Set is that the decision is theirs.

## 4. Propose it as one Question Set

One Set, one Question per comment you would do something about, and nothing
beside them: what a batch session sends is an ordinary Question Set, the same
shape as every other ask.

```yaml
title: What was said on the rate limiter's pull request
preface: |
  Two things you said I would change. Everything else I have answered here.

  Each one lists the ways I would do it — pick the way you want and I do it that
  way here, before I push. **Leave it** and I will not raise it again. Anything
  you write beside an answer is part of what I do about it.
questions:
  - label: Q1
    text: |
      You said on `window.rs` line 12 that the reset is the wrong way round. It
      is: the counter is cleared after the window is compared rather than
      before, so the first request of a window is counted against the last one.
    options:
      - n: 1
        text: Do it — move the reset above the comparison
        recommended: true
      - n: 2
        text: Leave it
  - label: Q2
    text: |
      You asked whether `limits.rs` still needs its own clock. It does not, so
      there are two ways to be rid of it.
    options:
      - n: 1
        text: Do it — collapse them onto `window.rs`'s clock
      - n: 2
        text: Do it — inject one clock at construction instead
      - n: 3
        text: Leave it
        recommended: true
```

- **Small.** This is a batch of comments rather than a reading of the branch:
  what belongs in the Set is what somebody has just asked for, and nothing you
  noticed while you were in there. Anything else is a finding for a review that
  is already over.
- **The Question is what the human reads**, on a phone, deciding. Say which
  comment it answers and what you would do about it, in prose — not as a patch.
  Their own words come back to them, so they can see them being taken up.
- **Each credible way to do it is an Option of its own**, worded in your own
  words: the Option says *which* way it is, because that is what they are
  picking between. Offer alternatives wherever more than one credible way
  exists, and only there — a comment with one sensible answer carries one Option
  for doing it, and a way invented to fill the Question out is one you would not
  defend.
- **Leave it is always offered**, on every Question, so declining stays possible
  whatever else it puts.
- **Recommend the one you would take** — the way you would do it, or leave it
  where that is your answer. One star per Question.
- **Nothing else goes on the Set.** There is no findings block and no marker
  saying which Option means do it: Verkstead reads how your session ended rather
  than a record of what you were answered. Which makes the answers yours alone
  to act on — nothing else holds an account of what each comment is about, so
  keep your own reading of it to hand, the file and the cause and what *done*
  would look like, for when they arrive.
- **Nothing is split out here.** Handing work on as a backlog belongs to the one
  review that read the branch whole. A comment asking for more than a batch
  session can do is worth saying so about in the Question, so the human can
  decline it knowing why — it is not a backlog for you to plan.
- **One Set for the batch** rather than one per comment.
- **Read `verkstead guide` before you write it** — how a Set is labelled, how
  much belongs in one, and the shape it goes over the wire in. It ships inside
  the binary, so nothing else has to be found.

Then put it through `verkstead ask`, run the way the Guide says to run one on
this backend: they answer in their own time, and that may be hours — they are on
a phone rather than at this terminal.

**The answers are yours to wait for, whichever way the Guide says to wait.**
Waiting is the ask working rather than the ask failing. Nothing ends this
session when the Set lands and nobody else is dispatched to act on it: what
becomes of what they said happens here, whether that means holding the ask open
or ending the turn and being told when they land. So there is nothing to do in
the meantime. Do not start on what you have only proposed, and do not take your
own recommendations.

If the ask itself fails — the server unreachable, any non-zero exit that is not
a refused Set — say so and stop. Never decide on their behalf.

## 5. Fix what they accepted

The Response is the whole of what you act on. Read all of it, the `comment` on
the Set included, before you touch anything: it is about the Set as a whole and
may reframe the answers above it.

- **A comment is accepted where they picked one of its do-it Options**, and
  *which* one they picked is part of the answer: do it the way they chose rather
  than the way you would have. Anything else is not a yes — leave it, an answer
  in their own words instead of a pick, a question left open.
- **What they wrote beside a yes is part of the instruction.** "Yes, but leave
  the public signature alone" changes what you do, and it is the reason their
  words come back to you at all.
- **One they declined is over.** Do not do it, do not do half of it, and do not
  raise it again.
- **Unanswered is not a yes.** Leave it as declined.

Fix each accepted one as what it is: the cause rather than the symptom, and
nothing beside what they agreed to. Anything else you notice on the way is
somebody else's piece of feedback. Then run the repository's tests and make sure
what you did works before it goes anywhere.

## 6. Commit it and push it

**Nothing waits on approval.** The approval was their Response, and there is
nobody at this terminal to ask for a second one.

One commit per thing you were asked for, so each reads against the decision that
asked for it:

    git add -A
    git commit -m "fix: <what was said, and what you did about it>"

Then push once, when the last of them is in:

    git push

In the worktree the comments named, which is where the fixes were made: those
are one repository's, and a commit made in the wrong directory is a change to
work nobody asked about.

Push, unlike most sessions here: this branch is already on a pull request, and a
fix that stays local is one nobody can see and nothing re-runs. The push is what
puts the commits in front of the checks again and in front of whoever left the
comment.

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

Do not open a pull request, do not merge anything, and do not touch any branch
beyond the one you were sent to: the branch checked out in the worktree the
comments named. Every other branch — in this repository and in every companion
beside it — belongs to somebody else's piece of feedback. The pull request
exists, and merging is the human's act.

Then say what you did and what you left, and stop.

## 7. A batch with nothing to do

Some batches ask for nothing: a question the commits since have already
answered, a note saying this reads well, somebody agreeing with a change that is
already on the branch.

**Ask nothing** where that is the whole batch.
**Say plainly, as the last thing you print, what was said and why none of it
needs a change** — that line is what the human sees on the Timeline, and it is
the only report that any of it was read at all.

A Set with nothing in it is a row for them to dismiss, and the point of asking
at all is to spend their attention only where there is a decision. Having
nothing to do is a fine outcome; inventing something so that something happened
is not.

The same holds at the other end: a batch whose every proposal was declined has
nothing to commit, and committing nothing is the right end to it. Say what you
put and what they left, and stop.
