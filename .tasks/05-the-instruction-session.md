# 05. The instruction session

## What to build

The other half of the Implementing target: a hand-written instruction, and the
session that carries it out.

The modal takes free text for this target — **required** where nothing stands to
continue, and available beside continuing where something does. Submitting lands
the instruction as the **Steer Event's body** and starts a session on it.

**It is a pipeline driver**, and that is what makes it different from the Manual
Task it replaces. It is registered as driving the Conversation, so nothing
sweeps the Conversation as standing still while it runs; it is judged by the
ordinary end-of-session rules, so one that ends badly stops the Conversation
with the ordinary Notice; and **on a clean finish the pipeline carries on from
whatever the branch then holds** — the wrap-up where a pull request exists, the
next task where the backlog holds one, and a stop that says so where neither
does.

**The steer records the direction as inline** where the Conversation had none: an
instruction is required exactly when nothing says how the work is being built,
and Resume refuses outright on a Conversation with no direction on it.

**A skill of its own.** Its own directory beside the other skills Verkstead
ships, reached by a const and a builder next to the manual task's, and launched
through a prompt variant of its own. Not the manual-task skill reused: that one
tells its session it is *outside whatever else the Conversation is doing* and
leaves the branch to the human, which is the opposite of a session the pipeline
carries on from. The skill says the session commits what it changed and stops,
and that what follows is the pipeline's rather than its own. The stage after
this one retires the manual-task skill, so this is what has to be left standing
— installed with the rest and launchable — before that happens.

## Acceptance criteria

- [ ] An instruction session that commits and goes quiet hands the pipeline on:
      the branch's pull request is wrapped up again, or the next task is worked.
- [ ] The session is registered as driving while it runs, and one that ends
      badly stops the Conversation with a Notice.
- [ ] The steer records the Conversation's direction as inline where it had
      none, and the instruction is on the Timeline as the Steer Event's body.
- [ ] The new skill is installed with the others and its own prompt is what the
      session is started on; the manual-task skill is left untouched.
