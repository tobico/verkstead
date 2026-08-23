# 05. The stall sweep

## What to build

The check that finds a Conversation nothing is driving, and raises the
Interruption that gives the human something to do about it.

**Stalled** is three things together: the state is Grilling, Implementing or
Wrapping; no driver is registered for it; and it has no open Interruption.
Draft, Direction, Done and Aborted are never stalled — nothing is supposed to be
driving them.

The sweep runs at server startup, every minute while it runs, and immediately
after a manual session ends. Startup matters most: a restart empties the
registries, so every genuinely stalled Conversation flags as the server comes
back, and every healthy one is picked up again by the resume paths that already
exist before the sweep ever looks.

Each stalled Conversation gets one Interruption, respecting the at-most-one-open
rule the store's index already enforces. Its evidence follows the existing shape
trimmed to what a stall has to say: the state the Conversation is in, that
nothing was running, what git makes of the Worktree, and the tail of the last
thing a session said where there is one. Nothing failed here, so the evidence
reads as a report of a Conversation standing still rather than of a crash.

A stall needs a step word of its own on the record, because what a Retry means
here is decided by the Conversation's state rather than by any backlog step —
which is the next two tasks. Until they land, Retry on a stall Interruption is
recorded and does nothing; *Take over manually* and *Abort* already mean exactly
what they always mean, and are the human's way out of a stall in the meantime.

## Acceptance criteria

- [ ] A Conversation that is Grilling, Implementing or Wrapping with no driver
      registered and no open Interruption gets one raised — at startup, and
      within a minute of stalling while the server runs
- [ ] Draft, Direction, Done and Aborted never flag; nor does a Wrapping
      Conversation under live watchers; nor does one that already has an
      Interruption open
- [ ] A manual session ending runs the check straight away
