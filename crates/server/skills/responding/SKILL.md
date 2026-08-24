---
name: responding
description: Answer a batch of comments left on a pull request: propose what to do about them as one Question Set, and land what the human accepts. Use when a session has been dispatched with fresh pull request comments as its feedback.
---

Answer what has just been said on this branch's pull request.
**You propose, and then you fix what was agreed to.** Nothing anybody wrote is
acted on before the human has said so, and everything they say yes to is done
here rather than by somebody else afterwards.

The comments are at the end of your prompt, under **What has just been said on
the pull request**: the batch whole, in the order it was said in, and where each
of it was said. A comment left on a line of the diff carries its file and line,
which is half of what it means.

The branch is already pushed and already has a pull request open. There is
nothing to create, nothing to switch to, and nothing to open.

## 1. Read what was said, and what it is about

Read the batch whole before you form an opinion about any part of it. It is one
person's thinking rather than a list — three replies in a minute are one point
being made — so the last of them often says what the first was getting at.

Then go and look. A comment names a symptom and the code says what the cause
is:

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

One Set, one Question per comment you would do something about, and a `review`
block that tells Verkstead which Option means *do it*:

```yaml
title: What was said on the rate limiter's pull request
preface: |
  Two things you said I would change. Everything else I have answered here.

  Answering **do it** has me do it here, before I push; **leave it** and I will
  not raise it again. Anything you write beside an answer is part of what I do
  about it.
questions:
  - label: Q1
    text: |
      You said on `window.rs` line 12 that the reset is the wrong way round. It
      is: the counter is cleared after the window is compared rather than
      before, so the first request of a window is counted against the last one.
      I would move the reset above the comparison and cover the roll-over in
      the tests.
    options:
      - n: 1
        text: Do it
        recommended: true
      - n: 2
        text: Leave it
  - label: Q2
    text: |
      You asked whether `limits.rs` still needs its own clock. It does not —
      `window.rs` has the better of the two — so I would collapse them onto
      one and update the tests that pin the other.
    options:
      - n: 1
        text: Do it
      - n: 2
        text: Leave it
        recommended: true
review:
  findings:
    - fix: Q1.1
      what: |
        `crates/limiter/src/window.rs` — `Window::count` is reset after the
        limit is compared rather than before, so the first request of a fresh
        window is refused against the window before it. Move the reset above
        the comparison, and cover the roll-over in the tests beside
        `counts_requests_in_a_window`.
    - fix: Q2.1
      what: |
        `crates/limiter/src/limits.rs` and `window.rs` each hold their own
        notion of now. Collapse them onto one clock — `window.rs` has the
        better of the two — and update the tests that pin the other.
```

- **Small.** This is a batch of comments rather than a reading of the branch:
  what belongs in the Set is what somebody has just asked for, and nothing you
  noticed while you were in there. Anything else is a finding for a review that
  is already over.
- **The Question is what the human reads**, on a phone, deciding. Say which
  comment it answers and what you would do about it, in prose — not as a patch.
  Their own words come back to them, so they can see them being taken up.
- **`what` is the work the answer authorises.** It is what you come back to when
  the answers arrive, and the only account of it anything other than you would
  have — so write it for a competent agent that has not read the comment: the
  file, the cause, and what *done* would look like. The comment is where it came
  from and not what it says to do.
- **`fix` names the Option that means do it** — `Q1.1` for a Question's, `Q1a.1`
  for a Sub-question's. It has to be an Option your Set actually offers, or the
  Set is refused: nothing else turns what was said into work.
- **No `split` here.** Splitting a finding out into a backlog belongs to the one
  review that read the branch whole. A comment asking for more than a batch
  session can do is worth saying so about in the Question, so the human can
  decline it knowing why — it is not a backlog for you to plan.
- **Every finding in the block is a Question in the Set**, and one Set for the
  batch rather than one per comment.
- **Read `verkstead guide` before you write it** — how a Set is labelled, how
  much belongs in one, and the shape it goes over the wire in. It ships inside
  the binary, so nothing else has to be found.

Then put it through `verkstead ask`, **as a background command**: it blocks
until they answer, and that may be hours — they are on a phone rather than at
this terminal.

**The answers are yours to wait for.** Nothing ends this session when the Set
lands and nobody else is dispatched to act on it: what becomes of what they said
happens here. So there is nothing to do while you wait. Do not start on what you
have only proposed, and do not take your own recommendations.

If the ask itself fails — the server unreachable, any non-zero exit that is not
a refused Set — say so and stop. Never decide on their behalf.

## 5. Fix what they accepted

The Response is the whole of what you act on. Read all of it, the `comment` on
the Set included, before you touch anything: it is about the Set as a whole and
may reframe the answers above it.

- **A comment is accepted only where they picked the Option you named as do
  it.** Anything else is not a yes: the other Option, an answer in their own
  words instead of a pick, a question left open.
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

Do not open a pull request, do not touch any other branch, and do not merge
anything. The pull request exists, and merging is the human's act.

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
