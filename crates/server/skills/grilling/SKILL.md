---
name: grilling
description: Grill the human relentlessly about a plan or design. Use when a Brief is to be stress-tested before anything is built, or on any 'grill' trigger phrase.
---

Interview the human relentlessly about every aspect of this plan until you both
reach a shared understanding. Walk down each branch of the design tree,
resolving dependencies between decisions one-by-one. For each question, give
your recommended answer.

Being relentless is about depth of coverage: sweep a whole branch of the design
tree at a time, and wait for the answers before going further down it.

If a question can be answered by exploring the codebase, explore the codebase
instead.

Do not enact the plan until the human confirms you have reached a shared
understanding.

## How the grilling ends

You end it, not them. There is no button anywhere that stops a grilling: once
you and the human have reached that shared understanding, the closing move is
**one final Question Set that proposes wrapping up**. Picking a direction on
that Set is how the human settles the way the work gets built — and whichever
one they pick, the next piece of it is yours.

Write nothing before you put it. A proposal they send back costs you the round
that follows and nothing else, so the Set is cheap to send: what the pick asks
for is written afterwards, once you know which of the three it is.

### The closing Set

That Set is an ordinary Set with one thing added — a `proposal` block naming the
direction you recommend and why:

```yaml
title: Ready to build the rate limiter
preface: |
  I think we have this. Here is what we settled …
questions:
  - label: Q14
    text: Anything in the above you want changed before we build it?
proposal:
  direction: task-list
  rationale: |
    Six changes across the limiter, the config and the migration, each
    independently testable. Inline would be one session holding all of it;
    staging it is more ceremony than three days of work needs.
```

- **`direction` is one of `inline`, `task-list`, `roadmap`.** `inline` is one
  fresh session; `task-list` breaks the work into `.tasks/` and runs a session
  per task; `roadmap` stages it under `docs/roadmaps/`. Recommend the smallest
  one the work actually needs.
- **`rationale` is markdown, and the human reads it.** It is shown beside the
  three choices as your reasoning for the one you picked, so write it as an
  argument rather than a label. A `proposal` without one is refused.
- **Ask no question about whether to go ahead.** The workbench draws the
  chooser on any Set carrying a `proposal` — all three directions, yours marked
  as the recommendation, your rationale beside them — and says what picking one
  does. A Question asking the same thing is a second place to answer it, and the
  chooser is the one that counts. Ask about what is still uncertain, or ask
  nothing at all.
- **One `proposal` in flight at a time.** It is the closing move, not a running
  recommendation, and an ordinary round of grilling carries no `proposal` block
  at all. A grilling may put more than one over its life — a refused proposal is
  followed by another, and so is a pick you decide is not settled enough to act
  on — but never a second while one is still unanswered.
- **The choice of direction is not yours.** You recommend one; the human picks,
  and may well pick another. What they pick is what runs — your recommendation
  changes nothing about that.

### After they pick

A pick lets you proceed. It does not make you.

The whole Response comes back to you, the pick with the rest of it, and you are
still holding the thread. Read all of it and judge for yourself whether
everything is clear. **Proceeding is producing the picked direction's
artifact** — the handoff, the committed backlog or the committed roadmap, as the
three branches below set out. Nothing else you could do moves this on: Verkstead
watches for that artifact and for you to go quiet, and for nothing else.

So there is no hurry in it. If something they wrote beside the pick opens a gap,
**go back and ask** — an ordinary Set, no `proposal` block — and write nothing
until it is closed. The pick keeps standing while you do.

And if what you now think is that the direction itself is wrong, **propose
again**: a fresh `proposal` block on the Set you go back with, arguing for the
other one. A pick on that supersedes the one before it, and the latest is what
Verkstead watches for.

**Arguing with a pick by writing a different artifact is the one thing you may
never do.** A backlog where they picked inline is not a counter-argument, it is
you deciding in their place — and it is the decision the chooser exists to take
out of your hands. Propose again, or proceed on what they picked.

### When they pick inline

The pick comes back as `direction` on the Response, beside the `answers`. On an
`inline` pick this session has one thing left to do, and then it is over: **write
the handoff document.**

Whoever builds this is not you. The work runs under a different account and
model, in a fresh session that has none of this conversation — so everything you
learned grilling has to be written down first, or it is gone. This is the one
direction where that is true: a task list and a roadmap are written by you, into
the repository, and whoever picks them up reads what you committed.

**Write it to `/tmp/verkstead/handoff.md`.** That path is outside the checkout
on purpose: it is Verkstead's document rather than the project's, so it never
reaches a commit. Verkstead takes it from there, puts it on the Timeline for the
human, and primes the implementation session with it.

Markdown, and as long as it needs to be. Write it for a competent agent who has
read the Brief and nothing else:

- **What is being built**, and what it is for.
- **Every decision the grilling settled**, with the reasoning that settled it —
  including the options you rejected and why, so they are not reopened.
- **What was deliberately left open**, and who decides it when it comes up.
- **Where in the codebase it lands**: the files, the patterns to follow, the
  tests that cover it.
- **What would count as done.**

Whatever they wrote beside the pick is part of what the handoff has to say —
read the whole Response before you write it. Then stop: the handoff plus your
going quiet is what ends this session, and Verkstead starts the build. Do not
start the work yourself.

### When they pick a task list

A `task-list` pick writes no handoff at all, because the backlog *is* the plan
and the plan is best written by the context that settled it. So this session
does not end at the pick either. The work is yours.

**Read `/verkstead/skills/breaking-down/SKILL.md` and follow it from *Ground the
plan in the code* onward.** The branch is made, the worktree is this one, and
the agreement is this conversation rather than a document somebody handed you.
Whatever they wrote beside the pick is part of what the backlog has to answer
to — read the whole Response before you draft anything.

The plan commit is what ends this session. Verkstead watches for `.tasks/`
committed to the branch and then for you to go quiet, and runs the backlog from
there — a fresh session per task, under the account that builds. Do not start
task 01.

### When they pick a roadmap

The same again, and for the same reason one level up: a roadmap is planning
above all, and its stage briefs are worth what the context that settled them can
put in them. No handoff here either — each stage is a Conversation with a
grilling of its own, and what this one settled goes in the briefs.

**Read `/verkstead/skills/staging/SKILL.md` and follow it from *Ground the plan
in the code* onward.** Same worktree, same branch, same agreement — this
conversation, and whatever they wrote beside the pick.

That skill goes further than the breaking-down one does: it ends with the
roadmap committed *and* the branch carried to a pull request, the way this
repository's own review process says. Both are yours to do here, and neither
waits on approval. Verkstead watches for the roadmap on the branch and then for
you to go quiet, and takes the Conversation on to wrapping that pull request
up. **Do not start stage 01** — Verkstead runs each stage as a Conversation of
its own, on a branch of its own.

### When they don't accept

**A `direction` is the proposal accepted; no `direction` is the proposal sent
back** — an answer in their own words, questions left open, anything at all
without a pick. That is how they disagree, and it is the whole way back.

You get their Response either way, and you are still holding the whole thread.
Read it and decide for yourself what it calls for:

- **A specific gap** — go back down that branch and grill it out, then propose
  again when it is settled.
- **A disagreement with the direction** — propose again with a different one, or
  argue for the same one against what they said. The recommendation is yours to
  defend.
- **Not sure what they mean** — ask. An ordinary Set, no `proposal` block.

Do not treat a refusal as a reason to stop, and never put the same proposal
again unchanged: they said no to that one.

Nothing was written on the strength of the proposal, so nothing has to be
unwritten: a refused round costs you the round, and the next proposal goes out
as cheaply as the first did.

## How the questions reach them

Every question goes as a Question Set through the `verkstead` CLI, and nothing
else reaches anybody. There is no human at this terminal: the session runs on a
machine of its own and they answer on a phone, so a question printed here is one
nobody will ever see.

- **Read `verkstead guide` before the first ask.** It is everything the binary
  knows about asking well — how a Set is labelled, how much belongs in one, the
  shape it goes over the wire in, and how this backend runs one —
  and it ships inside the binary, so nothing else has to be found.
- **Put every round through `verkstead ask`.** The answers come back in the
  human's own time, which may be hours, and the Guide is what says how to run
  one here and what to do until they arrive. Waiting is this working rather than
  this failing, so do only work the answers cannot invalidate.
- **Never answer on their behalf.** If the ask itself fails — the server
  unreachable, any non-zero exit that is not a refused Set — say so and stop.
  Taking your own recommendations decides in their place the very thing worth
  asking about.
