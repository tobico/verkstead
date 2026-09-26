# 05. Back where it came from

## What to build

**A steered Investigating ends in the state it was steered from, and that state
is driven again.** Task 03 made every Investigating end Done; this reads the
record task 04 writes and sends a steered one home instead.

**Where it goes.** The newest steer into Investigating on the Timeline is what
this Investigating came in through, and the source state written beside that Steer
Event is where it goes back to — nothing else about the Conversation changed, its
settles and its pull request exactly as they were. Two states are never returned
to, each having a way in of its own: an Investigating steered from a Draft or from
a closed Conversation ends **Done**, as an Investigate Conversation does. So does
one steered from Done, which is the same answer by the ordinary rule. And an
Investigating with no steer above it is an Investigate Conversation and ends Done,
which is task 03's path unchanged.

**And it is driven again.** A Conversation landed in Wrapping or Implementing with
nothing driving it is exactly what the stall sweep raises a stop about, so the
ending drives whatever state it lands in the way a pressed Resume drives that
state — which is already written, already exhaustive over the states, and already
the answer for a follow-up landing back in its wrap-up. Nothing is dispatched for
a landing in Done, there being nothing to drive. The registration is held until
whatever takes over has one of its own, as every driver here holds it.

**The store's move takes the landing state as a parameter**, the way the
follow-up's two endings share one move with the state as the word that differs,
and refuses for anything but Investigating. The checks are *not* put back to
waiting on the way to Wrapping: an investigation commits nothing and pushes
nothing, so it has given GitHub no new run to make up its mind about, and the
settle standing over it is still earned.

## Acceptance criteria

- [ ] A Wrapping Conversation steered into Investigating, marked and signalled,
      reads **Wrapping** again with its review settle, its comment settle and its
      pull request exactly as they were, and something driving it.
- [ ] One steered into Investigating from a Draft reads **Done**, and so does one
      steered in from a closed Conversation.
- [ ] An Investigate Conversation that was never steered still reads **Done**,
      with nothing dispatched.
- [ ] The branch holds no new commits in any of those cases, and the signal is
      still accepted over a dirty Worktree.
