# 06. Restarts resume on their own

## What to build

On startup, every Conversation in a driven state that is not deliberately
halted resumes through the same recompute Resume presses — Implementing
included, which today only a human can bring back. Circumstance halts and
Conversations a restart left undriven start driving again unasked; a
deliberate halt keeps its badge and waits for the press.

This replaces the restart-shaped Interruption: a grilling session the
restart killed resumes as a fresh grilling from the digest rather than
raising anything. The startup resume runs before the first stall sweep, as
today's resume paths do, so the sweep never halts what startup was about to
revive. A resume that refuses (nothing startable) halts with its refusal as
the Notice, so a broken Conversation surfaces instead of looping.

## Acceptance criteria

- [ ] After a restart, Grilling, Implementing and Wrapping conversations
      without a deliberate halt are driven again with no human press.
- [ ] A deliberately halted Conversation stays halted across a restart,
      badge intact.
- [ ] A startup resume that refuses leaves one halt with the refusal in its
      Notice, not a sweep loop.
