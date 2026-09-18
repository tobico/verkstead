# 01. The idle condition

## What to build

A session that has gone idle past the grace, with nothing open on its
Conversation and nothing to show for itself, reads on the workbench as what it
is: parked. Today it reads exactly like one hard at work — the sidebar card
carries the same mark and the status button says *Running* and nothing more,
deliberately, because a quiet session was held to be the run still doing its
job. The reporter watched one for ten minutes and concluded nothing had been
captured.

What is added is a **condition**, the kind *Waiting on checks* is: something
true *of* a state, drawn beside the lifecycle word rather than in place of it,
stored nowhere. It says how long the session has been idle and how many times
the Rescue has spoken to it — *idle 4 min, spoken to once* — which is what the
rescue loop already knows and nothing else does.

The Rescue stays off the Timeline as an event. It is Verkstead prodding an agent
rather than anything the work has got to, and CONTEXT.md says so; the line it
types is in the session's own Capture and that is unchanged.

**Where the two facts come from.** The rescue loop holds both in the driver: the
idle clock is the `Idle` the session register already carries, and the count of
times it has typed its line is a local of the loop. The register is what the
sidebar is drawn from — it is read once for the whole list rather than per row,
which is the shape the working and idle reads already take — so the count wants
a home beside the session on the register, bumped where the loop types its line.
Nothing is written to the store.

**Idle past the grace is not the idle flag already on the wire.** That flag is
the short, backend-specific silence a printing session is judged by — three
seconds for claude — and it is what the card's existing mark is drawn from. The
condition is the rescue's own reading: the session's idle duration measured
against the runner's grace, which is the same threshold the rescue arms on.

**And it goes when the session speaks or the run ends.** Output or a keystroke
puts the whole grace back on the clock, which the idle clock already does by
itself; a session no longer on the register has no condition at all.

The words are the conditions module's in the workbench, said once — the same
condition is drawn on the card the human opens and on the sidebar row they find
it by, and a condition worded two ways is two conditions to whoever reads them.
The sidebar row draws no state in words, so what it comes out as there is the
label read aloud, exactly as *Waiting on checks* does.

CONTEXT.md names the condition where it names the Rescue.

## Acceptance criteria

- [ ] A running session idle past the grace, with no Set open on its
      Conversation and nothing landed, shows the condition on both the sidebar
      row and the card; the count rises as the Rescue types its line again.
- [ ] Output from the session, a keystroke typed into it, or the run ending
      clears the condition.
- [ ] The count is held on the session register and nowhere in the store, and
      the condition's words live in the workbench's conditions module in one
      place, drawn from there by both the card and the row.
- [ ] CONTEXT.md names the condition alongside the Rescue, and says it is a
      condition rather than an Event.
