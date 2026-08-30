# 04. The nudge

## What to build

The far end of a store-and-nudge ask: the Response lands, and the session that
asked is told to come and get it. A session that ended its turn is not
listening for anything — Verkstead reaches it the one way there is, by typing
into its terminal, which is the channel Rescue already uses and the same channel
a watcher's keystrokes take.

**One canned line, and the Enter behind it.** Written as the human would write
it, naming the Set and the command that fetches it, so an agent that reads it
knows what to run without going back to the Guide. It takes the same path a
rescue takes and pays the same two costs that path has already been taught: the
line and its carriage return are typed a moment apart, because an interface
reads a burst as a paste and a paste's return is a line break rather than a
send; and the terminal echoes what is typed, so anything read straight back is
the keyboard rather than the session.

**Only where there is a session to nudge, and only for a Set it is idling on.**
Asked of the process rather than of the register, as a rescue is: a session that
has ended stays registered through its last sweep of the branch, and a line
typed in over that stretch goes into a terminal nothing is reading. A Response
to a `--deferred` Set types nothing, whatever backend it was asked on — nobody
is idling on one. A Response to a blocking Set types nothing either; the wait
delivers it.

**One place, however the Response arrived.** The human answers from the viewer
and an agent's Response could arrive through the agent API; both store it the
same way and both must nudge the same way, so this hangs off the one moment a
Set is settled rather than off either caller.

**A session that has gone is the folding rule's case, unchanged.** Its Answers
go into the next session's prompt of that Conversation, oldest first, under the
documents the prompt is built from, folded once and recorded as folded — all of
which is what a stored Set already does. Nothing about the folding is touched
here; what this task shows is that the two ends do not overlap.

Nothing goes on the Timeline for the nudge. It is Verkstead speaking to an
agent rather than anything the work has got to, and the line is in the session's
own Capture — the same account the rescue gives of itself. CONTEXT.md gains the
new kind of ask beside the Blocking and Deferred ones, and says plainly that
the nudge in *store-and-nudge* is this line rather than the viewer's **Nudge**.

## Acceptance criteria

- [ ] The stub session receives the line, runs `verkstead answers` and carries
      on with what the human said; the line and the Enter arrive as two
      keystrokes and the Set is named in it.
- [ ] A stub killed before the Response lands has those Answers under the next
      session's prompt, folded once, and nothing is typed anywhere.
- [ ] A Response to a `--deferred` Set on the same backend, and to a blocking
      Set on Claude, types nothing into any terminal.
