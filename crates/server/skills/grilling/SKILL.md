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
you and the human have reached that shared understanding, the last thing you do
is put **one final Question Set that proposes wrapping up**, and answering it is
what moves the work on to being built.

That Set is an ordinary Set with one thing added — a `proposal` block naming the
direction you recommend and why:

```yaml
title: Ready to build the rate limiter
preface: |
  I think we have this. Here is what we settled …

  Answering **yes** ends the grilling and hands over to the build. Anything
  else keeps us here — say what is still open and I will pick it up.
questions:
  - label: Q14
    text: Ready to build it this way?
    options:
      - n: 1
        text: Yes, go ahead
        recommended: true
      - n: 2
        text: Not yet — more to work through
proposal:
  direction: task-list
  accepted_by: Q14.1
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
- **`accepted_by` names the Option that means *go ahead*** — `Q14.1` for a
  Question's, `Q14a.1` for a Sub-question's. It has to be an Option your Set
  actually offers, or the Set is refused: nothing else can end the grilling, so
  a proposal nobody can accept is one that would grill forever.
- **Put a `proposal` on one Set and no others.** It is the closing move, not a
  running recommendation; an ordinary round of grilling carries no `proposal`
  block at all.
- **The choice of direction is not yours.** You recommend one; the human picks
  afterwards, and may well pick another. Nothing about the Answer to your
  question changes that.

### When they don't accept

Any other Answer keeps the Conversation grilling: a different Option, an answer
in their own words instead of an Option, or the question left open. That is how
they disagree, and it is the only way back — so **say in the Preface what
answering yes does**, as the example does. A human who did not realise the
Option ended the grilling cannot un-answer it.

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
