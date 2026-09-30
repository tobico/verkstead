# 02. The Set ids across

## What to build

A resumed agent remembers asking a Question Set and remembers waiting on it.
Both are gone: the wait was a shell command that died with the process, and the
Set itself landed on this device under **a new id**, every id in a transferred
record being renumbered as it lands. So the note tells it the new ids, and the
Set is left open for it to come back to.

**The map already exists and is thrown away.** The landing of a record builds
old Set id to new as it walks, because everything pointing at a Set has to be
renumbered against it, and drops the map when the transaction commits. The
record lands in a leg of its own, one before the arrival, so the map cannot be
handed to the resume in memory: it is written down against the Conversation as
the record lands, and read back when the session is started.

**Only the ones the agent had open.** A Set the human answered before the move
is one the agent has already read, and a Deferred Ask never had anybody waiting
on it. What goes in the note is the Sets the gone session was idling on — the
same reading a relaunch makes to decide what to orphan — each by the id it now
has, with the line that says how to come back for the answers. The command
that does that already exists and already handles exactly this case: a blocking
ask whose wait was killed comes back by id, and a Set nobody has answered yet
is a refusal rather than something to idle on.

**And they are left open rather than locked.** This is the reversal. A
relaunched grilling locks every Set it orphans, and stage 09's arrival does the
same, because the session that asked has gone and the human would be answering
into nothing. A harness resume brings that reader back on another machine, so
the premise does not hold: the Set stays open, the human's answer reaches the
resumed session, and nobody is asked the same question twice. **Conditional on
the resume actually happening** — a fallback to Verkstead's Resume locks them
as it does today, because there the reader really has gone.

The note's Set lines are the other half of what the human sees: a Set they were
part way through answering when the work moved is still there on the Timeline,
answerable, rather than struck through and asked again from the new machine.

## Acceptance criteria

- [ ] `verkstead answers <new id>` on the receiving device returns the answers
      to the Set that was asked on the sending one.
- [ ] A Set open at the moment of the move is still open after it, and an
      answer given afterwards reaches the resumed session.
- [ ] The note names each Set the moved session was idling on by its new id,
      and no Deferred Ask or already-answered Set is in it.
- [ ] Where the harness resume does not happen, the open Sets are locked
      unanswered exactly as they are today.
