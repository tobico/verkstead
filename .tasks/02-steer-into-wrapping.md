# 02. Steer into Wrapping

## What to build

The resume half of the press, on the second target that carries no payload:
submitting clears any stop, recreates what is missing, moves the Conversation
and sets the wrap-up going in the same press.

**Wrapping is offered only where the record already holds a pull request.** A
wrapping Conversation is defined by the pull request under it — the store writes
the move and the pull-request row as one act, so a Wrapping with nothing under
it cannot exist. A steer into Wrapping is therefore a move onto a pull request
that is already there. Where the Conversation has none, the target is not
offered at all, and a submit that names it anyway is refused by name.

What the submit does, in order: force-stop the running session where **Interrupt
current task** was ticked; recreate the Worktree from the branch where the
directory it names has gone; **clear the stop** — the one the click wrote, and
any Stop asked for that has not landed yet — move the Conversation; record the
Steer Event; and set the wrap-up's watchers going afresh, with the fix attempts
forgotten. That last part is what a pressed Resume already does for a wrapping
Conversation, and it is reused rather than forked.

Clearing the stop goes last of the things done before anything is launched: the
run does not advance past a stop, so a launch over one would find the
Conversation stopped and start nothing.

**The Pairing** for the role steered into is shown in the modal, prefilled from
the Conversation's own implementation Pairing, and what is picked is **recorded
as the Conversation's** — steering re-settles what runs the work rather than
picking for one session. A Conversation with none fixed yet has the pick as part
of the modal rather than as an error path.

## Acceptance criteria

- [ ] Steering a Conversation whose Worktree directory is gone recreates it from
      the branch and proceeds.
- [ ] A wrapping Conversation steered into Wrapping starts its watchers over
      with the fix attempts forgotten, and the stop written at click is gone.
- [ ] Wrapping is not offered where the Conversation has no pull request
      recorded, and a submit naming it is refused by name.
- [ ] The Pairing picked in the modal is recorded as the Conversation's
      implementation Pairing.
