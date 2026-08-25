# 02. Retire Reopen

## What to build

Reopen goes, and steering is the one way back into a Conversation Verkstead has
finished with. The press, its endpoint, its outcome type and the store call
behind it are all removed, along with the tests that only existed to cover them.

Nothing replaces it, because Steer already is the replacement and already
works: every state is a source, and a steer checks a Worktree out of the branch
where the directory has gone. A steer into Grilling opens a new round with a new
Brief frozen where it lands — which is the whole of what Reopen did, minus the
second door to keep true.

**The workbench draws Steer where Reopen used to be drawn.** Reopen was the one
control offered at the finished end of the ladder, sitting beside *Start
grilling* under the Timeline. Steer is already in the Conversation's menu and is
already drawn whatever state the Conversation is in, so what this needs is the
removal and a check that the finished end of the ladder still reads as having
something to do from here.

Doc comments across the store and the render crate explain the frozen Brief, the
second round and the recorded Worktree by pointing at reopening. Those all have
a steer to point at instead — the mechanism is the same one, under the control
that survives.

## Acceptance criteria

- [ ] Nothing in the product offers Reopen: no press, no endpoint, no exported
      outcome type, and no store call.
- [ ] A **closed** Conversation steered into Grilling gets a Worktree checked out
      of its branch, a new round, and a new frozen Brief — demonstrated end to
      end, not asserted in a unit test alone.
- [ ] A **Done** Conversation steered into Grilling does the same, and the
      Timeline shows the Steer Event and the move beneath it.
