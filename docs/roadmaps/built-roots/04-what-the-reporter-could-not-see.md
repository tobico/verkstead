# 04. What the reporter could not see

## Goal

Three things a first Windows run left invisible are said where the human looks.
A session that has gone idle and been spoken to carries a condition on its row
— *idle 4 min, spoken to once* — so a parked agent and a working one no longer
read alike. A boundary being written is a line in the session's Capture when it
starts and when it ends, and the log's per-entry lines add up to its total. A
stop Notice whose session said nothing says how it ended — *exited with code 1
after 0.4 s, having printed nothing*.

Depends on no other stage.

## Decisions in force

- **The Rescue stays off the Timeline as an event**, as CONTEXT.md says: it is
  Verkstead prodding an agent rather than anything the work has got to. What is
  added is a **condition**, the kind *Waiting on checks* is — true *of* a state,
  drawn beside the lifecycle word on the card and the sidebar row, stored
  nowhere. Rejected: a Timeline event per Rescue; and leaving it, on the grounds
  that the Screen shows it and the stop comes within fifteen minutes — the
  reporter watched a parked session for ten and concluded nothing was captured.
- **What the condition says** is how long the session has been idle and how
  many times it has been spoken to, which is what the rescue loop already
  knows; it goes when the session speaks or the run ends. Its words are the
  conditions module's, said once.
- **The Capture is where the boundary line goes.** The Agent run event exists
  before the boundary is written and bwrap's own complaints already land in the
  Capture, so a line at the start — how many entries — and one at the end — how
  long — is the session's own record saying what it was waiting on. Written only
  on the platform that writes entries. Rejected: a Notice, which is a stop; and
  the log alone, which nobody opens from a phone.
- **The accounting is fixed regardless.** Sixty entries each under the
  quarter-second threshold cannot make 112 s, so the time went somewhere the
  per-entry line does not see — the read that checks whether an entry already
  stands, the steps, or the record. Every entry's duration is logged at debug
  and the total names the count, and the stage finds the gap on a Windows
  machine.
- **The stop Notice gains one sentence** under *What the last session said*
  when the Transcript and the Capture are both empty: the exit code and the
  session's lifetime, which the relay knows as the child ends. A Notice that
  said "it said nothing at all" was true and left an hour of diagnosis to the
  human; an instant exit named as an instant exit points at the binary.

## Proposed tasks (provisional)

1. **The idle condition.** The rescue loop's idle-since and count reach the
   UI's condition fields; the card and the row draw it; CONTEXT.md names it.
   AC: a session idle past the grace with no Set open shows the condition;
   a keystroke or output clears it; the words are in one place.
2. **The boundary lines in the Capture.** AC: a Windows session's Capture opens
   with the start line and carries the end line before the agent's first byte;
   a Linux session's carries neither.
3. **The accounting.** AC: the sum of the per-entry durations and the boundary
   total agree in the log; whatever made the difference is named in the log
   line or fixed.
4. **The Notice's last sentence.** AC: a stub session that exits at once with
   nothing printed yields a Notice naming its exit code and lifetime; one that
   spoke is unchanged.

## Re-verify at start

- Assumes the Rescue still holds its idle clock and count in the driver rather
  than in the store, and that conditions are still derived in the UI API from
  the Conversation's state.
- Assumes the Agent run event is still recorded before `Sandbox::command` writes
  the boundary.
- Assumes the relay still learns the child's exit status and that the stop
  Notice is still composed from the Transcript with the Capture as fallback.
