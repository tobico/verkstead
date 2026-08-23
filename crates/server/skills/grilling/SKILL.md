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
**the handoff document, and then one final Question Set that proposes wrapping
up**. Picking a direction on that Set is what moves the work on to being built.

### The handoff document

Whoever builds this is not you. The work runs under a different account and
model, in a fresh session that has none of this conversation — so everything you
learned grilling has to be written down first, or it is gone.

**Write it to `/tmp/verkstead/handoff.md`, before you put the closing Set.**
That path is outside the checkout on purpose: it is Verkstead's document rather
than the project's, so it never reaches a commit. Verkstead takes it from there,
puts it on the Timeline for the human, and primes the implementation session
with it.

Markdown, and as long as it needs to be. Write it for a competent agent who has
read the Brief and nothing else:

- **What is being built**, and what it is for.
- **Every decision the grilling settled**, with the reasoning that settled it —
  including the options you rejected and why, so they are not reopened.
- **What was deliberately left open**, and who decides it when it comes up.
- **Where in the codebase it lands**: the files, the patterns to follow, the
  tests that cover it.
- **What would count as done.**

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
- **Put a `proposal` on one Set and no others.** It is the closing move, not a
  running recommendation; an ordinary round of grilling carries no `proposal`
  block at all.
- **The choice of direction is not yours.** You recommend one; the human picks,
  and may well pick another. What they pick is what runs — your recommendation
  changes nothing about that.

### When they don't accept

The pick comes back as `direction` on the Response, beside the `answers`.
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

**Rewrite the handoff before you propose again.** Whatever came out of the round
that followed the refusal is part of what was settled, and the handoff standing
where it is would hand over the understanding you had before they disagreed.

## How the questions reach them

Every question goes as a Question Set through the `verkstead` CLI, and nothing
else reaches anybody. There is no human at this terminal: the session runs on a
machine of its own and they answer on a phone, so a question printed here is one
nobody will ever see.

- **Read `verkstead guide` before the first ask.** It is everything the binary
  knows about asking well — how a Set is labelled, how much belongs in one, and
  the shape it goes over the wire in — and it ships inside the binary, so
  nothing else has to be found.
- **Put every round through `verkstead ask`.** It blocks until the answers come
  back, which may be hours. Idling is this working rather than this failing, so
  run it as a background command and do only work the answers cannot invalidate
  while you wait.
- **Never answer on their behalf.** If the ask itself fails — the server
  unreachable, any non-zero exit that is not a refused Set — say so and stop.
  Taking your own recommendations decides in their place the very thing worth
  asking about.
