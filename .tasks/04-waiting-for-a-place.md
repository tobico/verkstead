# 04. Waiting for a place

## What to build

A stage that is **ready and has nowhere to run** reads *waiting for a place*: every
stage it stands on has settled, nothing is halted about it, and what holds it is
one of the two limits — how many stages of its own roadmap run at once, and how
many Conversations the whole server runs at once.

Told apart from waiting on a stage, which is the distinction that earns this its
own word: a stage waiting on a dependency waits on work, and a stage waiting for a
place waits on the machine. A roadmap gone quiet with ready work in it would
otherwise read, on the card, as a roadmap the scheduler forgot.

**Nothing about the waiting is stored, and nothing here starts storing it.** What
is ready is worked out afresh from the declarations, the record and the boxes
every time it is asked — which is how the look that spends freed places already
works — and both limits are read from Settings as the scheduler reads them, so a
machine that has raised one shows fewer stages waiting. A second record saying
*this stage is waiting* could only come to disagree with a readiness worked out
afresh.

**One word for both limits.** The sentence on the Timeline names which of the two
holds a stage, because a reader's next move differs — the roadmap's is waited out
by a stage of that roadmap settling and the server's by anything anywhere coming
free. A card row has no room for that, and the Timeline of the Conversation that
held it already says it.

Where more stages are ready than there are places, which of them are waiting is
the scheduler's own order: the lowest-numbered take the places and the rest wait.
So a roadmap with one place left and three ready stages shows one *in progress*
and two *waiting for a place* rather than three of either.

## Acceptance criteria

- [ ] A ready stage held by its roadmap's own limit, and one held by the server's,
      both read *waiting for a place* on the card and in the pane.
- [ ] Such a stage is told apart from one waiting on a stage: with its dependencies
      settled it never reads *waiting on*, and with one unsettled it never reads
      *waiting for a place*.
- [ ] Both limits come from Settings, so raising one moves a stage out of this
      state without anything else changing.
