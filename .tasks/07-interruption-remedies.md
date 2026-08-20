# 07. Interruption remedies

## What to build

An unattended run stops when something goes wrong, and stopping has to be
legible. An Interruption is something Verkstead detected but cannot resolve
itself — a session that exited badly, or one that ended having landed nothing —
and it becomes a Timeline Event carrying the evidence, with the three remedies
as actions in the GUI.

Roadrunner asked this over askance because nobody was at its terminal. Here the
Timeline *is* where the human looks, so the same question is GUI-native: the
Event itself offers the choice, and the Conversation carries the *blocked on
you* badge while it waits. The session idles rather than being torn down.

The remedies are roadrunner's, and so is what they mean:

- **retry** — run the step again in a fresh session, told whatever the human
  writes alongside, so "try again but leave that one alone" reaches the agent
  that can act on it.
- **take over manually** — Verkstead stops driving so the human can take the
  step on.
- **abort** — the run ends here.

In every case the repo is left as the session left it.

The evidence is what makes the choice answerable without opening a terminal:
which step failed, how it ended, what git makes of the Worktree, and the tail of
what the session last said — enough to see what went wrong, short of a wall of
text on a phone.

Detection covers a bad exit and a session that ends having landed nothing.
Usage limits — an account exhausting its window mid-run — are **out of scope**
for this stage.

## Acceptance criteria

- [ ] A session exiting badly, or ending with nothing landed, produces an
      Interruption event
- [ ] The event carries the failing step, how it ended, the Worktree's git
      status and the tail of the session's output
- [ ] Retry, take over manually and abort are all offered on the event
- [ ] Retry launches a fresh session for the same step, carrying the human's
      note to it
- [ ] Take over manually stops Verkstead driving; abort ends the run
- [ ] The Conversation shows *blocked on you* while an Interruption is open, and
      the run does not advance past it
