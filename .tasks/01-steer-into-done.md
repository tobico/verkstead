# 01. Steer into Done

## What to build

The Steer control's first end: a row in the Conversation's menu beside Stop, the
modal it opens, and the one target that needs no payload.

**Clicking Steer stops the drive before the modal opens.** Nothing new launches,
and a session that is running is seen out — the same stop the Stop press writes,
and deliberate: the click freezes the world while the human composes. The
modal's **Interrupt current task** checkbox ends that session where it stands
instead. **Cancel leaves the Conversation stopped**, with Resume on offer; that
is accepted rather than a bug.

The modal carries a target picker. Only **Done** is offered in this task — the
other three targets arrive in the tasks after it, each with the payload it
needs. No Pairing is picked on a steer into Done: nothing runs, so there is
nothing to settle.

Submitting moves the Conversation to Done and lands **two** Events. The **Steer
Event** is its own kind, carrying the target and (from later tasks) the payload:
the human moved it, and that is a different record from the machine's plain
**Moved** line, which is written beside it as it is for every other move. Nothing
resumes — there is nothing to drive in Done, and a steer to Done is the move
alone.

Reachable from any state. Into Done nothing has to be recreated, because nothing
runs; recreating a missing Worktree or branch belongs to the tasks that start
something.

Old Timelines stay readable, and so must new ones on an older build's terms: an
Event kind the store cannot read is an error, so the new kind is added to the
reader as well as the writer.

Reopen is still on a Done Conversation until the stage after this one retires
it. Steer must stand beside it without colliding.

## Acceptance criteria

- [ ] Clicking Steer stops the drive before the modal opens, and Cancel leaves
      the Conversation stopped with Resume drawn on it.
- [ ] Interrupt current task ends the running session where it stands; without
      it the session is seen out and nothing new launches.
- [ ] Submitting into Done moves the Conversation, leaves a Steer Event carrying
      the target beside the machine's Moved line, and starts nothing.
- [ ] Reopen still works on a Done Conversation.
