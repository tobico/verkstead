---
name: reviewing
description: Review a branch that is already on a pull request, propose the fixes it and its comments need as one Question Set, and land the ones the human accepts. Use when a session has been dispatched to review work at the end of a wrap-up.
---

Review the branch this worktree is on, put what you find to the human, and fix
what they accept. **You propose, and then you fix what was agreed to.** Nothing
you find is changed before they have said so, and everything they say yes to is
changed here rather than by somebody else afterwards.

You are the first thing to see this work whole. The sessions that wrote it each
saw one task and none of them saw the branch, and you have none of their
context — which is the point. Read it as somebody who has to live with it.

What you propose about is the branch **and** whatever has already been said on
its pull request. Nobody else is sent to act on those comments: what they ask for
goes into your Set beside what you found yourself, so the human decides about
their own words rather than watching a session act on them unasked.

The branch is already pushed and already has an open pull request. There is
nothing to create, nothing to switch to, and nothing to open.

## 1. Read the work

Read the whole diff first, before you form an opinion about any part of it:

    gh pr diff

Then go and read what it landed in. A diff shows what changed and hides what it
changed *around* — the callers, the sibling module doing the same job a
different way, the test that should have caught this and does not.

Read what the repository says about itself, too: its `CLAUDE.md` or
`AGENTS.md`, the docs it keeps for agents, the conventions its neighbouring code
actually follows. The work is meant to look like it belongs here.

The Brief and the handoff in your prompt say what the work was *for*. Review
against those rather than against what you would have built.

## 2. Read what has already been said

Where anything had been said on the pull request before you started, it is under
**What has been said on the pull request** at the end of your prompt: the
comments whole, in the order they were said in, and where each of them was said.
A comment left on a line of the diff carries its file and line, which is half of
what it means.

Read it as what it is — somebody who has read this branch, telling whoever wrote
it what they think — and go and look at what each one is about before you decide
what it is asking for.

**You are the only session that will act on these.** Nothing else is dispatched
about them, so a comment you leave out is a comment nobody answers. Work out what
each is asking for and carry it into your Set with everything else you found: one
Question, in your own words, saying what you would do about it. Some ask for a
change, some are a question you can answer in the Question's own text, and some
are somebody saying they are happy — that last is nothing to propose, and
inventing work out of agreement is not answering it.

What you must not do is act on one because it is the human talking. A comment is
still a proposal until they have said yes to *this* reading of it: the words on a
pull request are not the same thing as an instruction to a session, and the
answer to "this is the wrong way round" is a decision they have not made yet.

## 3. What is worth raising

The seams are where this session earns its context. A session per task cannot
see across tasks, so look hardest at what only shows up from here:

- **Correctness** — the bug, the unhandled case, the thing that is wrong.
- **Seams between the pieces** — two tasks that solved the same problem twice,
  an abstraction introduced by one and ignored by the next, a half-done rename.
- **Drift from what was settled** — where the branch quietly decided something
  the handoff had already decided differently.
- **Tests that do not test** — the assertion that passes whatever the code does,
  the case nobody covered.
- **What is now stale** — docs, comments and names that describe the code as it
  was before this branch.

Raise what is worth a human's decision, and nothing else. Style you would have
done differently, a name you would have picked, a refactor nobody asked for —
these cost a human a decision each and buy nothing. **If you would not defend it
in a review, do not raise it.**

## 4. Change nothing yet

No edits, no commits, no pushes, no `gh` command that writes anything — not
until the human has answered. Fixing your own findings before they have seen
them is deciding in their place, and the whole point of the Set is that the
decision is theirs.

## 5. Propose it as one Question Set

One Set, one Question per finding, and a `review` block that tells Verkstead
which Option means *fix it*:

```yaml
title: Review of the rate limiter branch
preface: |
  Three things worth a decision. Everything else looks right to me.

  Answering **fix it** has me fix it here, before I push; **leave it** and I
  will not raise it again. Anything you write beside an answer is part of what
  I do about it.
questions:
  - label: Q1
    text: |
      The window counter is never reset between windows, so a client that goes
      quiet for an hour is still refused. `window.rs` counts from the first
      request and nothing clears it.
    options:
      - n: 1
        text: Fix it
        recommended: true
      - n: 2
        text: Leave it
  - label: Q2
    text: |
      `limits.rs` and `window.rs` each grew their own clock. Two now, and the
      tests pin both.
    options:
      - n: 1
        text: Fix it
      - n: 2
        text: Leave it
        recommended: true
review:
  findings:
    - fix: Q1.1
      what: |
        `crates/limiter/src/window.rs` — `Window::count` accumulates for the
        life of the process and is never reset when the window rolls over, so a
        client that exceeds the limit is refused for ever rather than for the
        window. Reset it as the window rolls, and cover the roll-over in the
        tests beside `counts_requests_in_a_window`.
    - fix: Q2.1
      what: |
        `crates/limiter/src/limits.rs` and `window.rs` each hold their own
        notion of now. Collapse them onto one clock — `window.rs` has the
        better of the two — and update the tests that pin the other.
```

- **One Question per finding**, and every one of them answerable: two Options,
  one meaning fix it and one meaning leave it. Recommend the one you would take.
- **The Question is what the human reads**, on a phone, deciding. Write it as
  prose that says what is wrong and why it matters — not as a patch.
- **`what` is the work the answer authorises.** It is what you come back to when
  the answers arrive, and the only account of the finding anything other than you
  would have — so write it for a competent agent that has not read the diff: the
  file, the cause, and what *done* would look like.
- **`fix` names the Option that means fix it** — `Q1.1` for a Question's,
  `Q1a.1` for a Sub-question's. It has to be an Option your Set actually offers,
  or the Set is refused: nothing else turns a finding into work.
- **Every finding in the block is a Question in the Set**, and the block goes on
  one Set and no others. This is the review, not a running commentary.
- **A comment's fix is a finding like any other**, in the same Set and the same
  block. Say in the Question which comment it answers and whose it was, so the
  human can see their own words being taken up — and write `what` for the agent
  that will do it, which is you: their comment is where it came from and not what
  it says to do.
- **Read `verkstead guide` before you write it** — how a Set is labelled, how
  much belongs in one, and the shape it goes over the wire in. It ships inside
  the binary, so nothing else has to be found. A review that has found more than
  a sitting's worth of decisions is a review that should raise the ones that
  matter.

Then put it through `verkstead ask`, **as a background command**: it blocks
until they answer, and that may be hours — they are on a phone rather than at
this terminal.

**The answers are yours to wait for.** Nothing ends this session when the Set
lands and nobody else is dispatched to act on it: what becomes of your findings
happens here. So there is nothing to do while you wait. Do not start on what you
have only proposed, and do not take your own recommendations.

If the ask itself fails — the server unreachable, any non-zero exit that is not
a refused Set — say so and stop. Never decide on their behalf.

## 6. Fix what they accepted

The Response is the whole of what you act on. Read all of it, the `comment` on
the Set included, before you touch anything: it is about the Set as a whole and
may reframe the answers above it.

- **A finding is accepted only where they picked the Option you named as fix
  it.** Anything else is not a yes: the other Option, an answer in their own
  words instead of a pick, a question left open.
- **What they wrote beside a yes is part of the instruction.** "Yes, but leave
  the public signature alone" changes what you do, and it is the reason their
  words come back to you at all.
- **A finding they declined is over.** Do not fix it, do not fix half of it, and
  do not raise it again.
- **Unanswered is not a yes.** Leave it as declined. Where it is one you
  genuinely cannot leave — the correctness bug the rest of the branch turns on —
  go back with one short Set about that alone and wait as before.

Fix each accepted finding as what it is: the cause rather than the symptom, and
nothing beside what they agreed to. Anything else you notice on the way is a
finding you did not raise, and fixing it now is a decision they did not get to
make. Then run the repository's tests and make sure what you did works before it
goes anywhere.

## 7. Commit it and push it

**Nothing waits on approval.** The approval was their Response, and there is
nobody at this terminal to ask for a second one.

One commit per finding, so each reads against the decision that asked for it:

    git add -A
    git commit -m "fix: <the finding, and what you did about it>"

Then push once, when the last of them is in:

    git push

Push, unlike most sessions here: this branch is already on a pull request, and a
fix that stays local is one nobody can see and nothing re-runs. The push is what
puts the commits in front of the checks again.

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

Then say what you fixed and what you left, and stop.

## 8. A review with nothing to raise

Nothing to raise means both halves: you found nothing yourself, *and* nothing
said on the pull request asks for anything. Comments you were given are the other
source of a decision here — a branch you would not have touched, where somebody
has asked for a change, is a review that proposes about that change alone.

Ask nothing where there is genuinely neither.
**Say plainly, as the last thing you print, that you reviewed the branch and
found nothing worth raising** — that line is what the human sees on the
Timeline — and stop. Say that you read what was said on the pull request too,
where there was anything to read: it is the only report that any of it was
looked at.

A Set with no findings in it is a row for them to dismiss, and the point of this
phase is to spend their attention only where there is a decision. Finding
nothing is a fine outcome; inventing a finding so that something happened is
not.

The same holds at the other end: a review whose every finding was declined has
nothing to commit, and committing nothing is the right end to it. Say what you
raised and what they left, and stop.
