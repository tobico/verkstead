# 03. The fifth steer target, the state, and the session it launches

## What to build

**Follow-up** as a Lifecycle state and a steer target, end to end: the steer
modal offers it, the submit records it and launches the session, and the
Conversation is honestly driven while it runs.

- A new `Lifecycle` variant, stored `follow-up`. A **driven state**: the
  stall sweep judges it by whether a driver is registered, exactly as it
  judges Implementing and Wrapping; Draft, Done and Closed stay the states
  nothing drives.
- A fifth `SteerTarget`. **Offered from Done and Wrapping only, and only
  where the record holds a pull request** — a steered-to-Done Draft has
  none, so the predicate is on the record, refused by name the way the
  Wrapping target's no-pull-request refusal is. It runs work, so the modal
  asks for a Pairing like the other running targets.
- **The brief is required**, and it is the Steer Event's own body — the
  shape an Implementing steer's instruction takes, not a Brief round of the
  Conversation. A submit without one is refused by name.
- The submit launches the session in turn in the Conversation's Worktree —
  remade from the branch where the directory has gone — on the following-up
  skill's prompt from task 02, under the implementation Pairing. The branch
  watcher records its commits as it records any session's; Stop, Force stop
  and the Steer click's own stop all behave as they do for other running
  states.
- **Interim end, until task 05 lands**: a follow-up session that ends, however
  it ends, stops the Conversation with a Notice. The real done rule replaces
  this arm in 05.

## Acceptance criteria

- [ ] Steering a Done Conversation that has a pull request into Follow-up
      records the Steer Event carrying the brief plus the Moved line,
      moves the state, and launches a session on the following-up skill in
      the Conversation's Worktree.
- [ ] The target is refused by name on a Conversation without a pull request
      and on a submit without a brief, and is not offered outside Done and
      Wrapping.
- [ ] A running follow-up is never swept as stalled, and its commits land on
      the Timeline.
