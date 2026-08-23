# 04. The driver registry

## What to build

What says a Conversation is being driven, so that the next task can say when one
is not.

A registry held in memory beside the sessions registry, keyed by Conversation.
The tasks that drive a Conversation register themselves for as long as they run
and are off it the moment they stop: the runner's loops — working a backlog,
following an inline run, following a roadmap — and the set of watchers a
wrapping Conversation has going. Deregistration has to survive a task dying as
well as returning, so a guard that comes off on drop is the natural shape;
a driver that panicked and left its registration behind would be a stalled
Conversation nothing could ever detect.

**A grilling has no such task.** Starting a grilling launches the session and
keeps nothing that follows it, so there is nothing there to hold a registration.
A Conversation that is Grilling therefore counts as driven exactly while a
session is registered for it — the sweep reads the sessions registry for that
state and the driver registry for the others.

This is judged by registration rather than by a time window on purpose. A
Wrapping Conversation idling with no session for days is its normal healthy
condition, so a window would either flag it for ever or leave dead watchers
undetectable. Registration also gives the restart behaviour for free: a
restarted server holds no registrations at all, which is exactly right, because
none of these tasks survive the process.

## Acceptance criteria

- [ ] A driver registers for the life of its task and is off the registry once
      it returns, and off it too when it panics
- [ ] The gaps between a live runner's sessions still read as driven, while a
      runner whose loop has ended does not
- [ ] A Grilling Conversation reads as driven exactly while a session is
      registered for it
- [ ] Covered by tests in the runner's own style, against a stand-in agent
      rather than a real one
