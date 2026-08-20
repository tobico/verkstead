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
