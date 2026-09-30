# 03. Ended at the turn's end

## What to build

Stage 09's mover waits for a session to **end on its own** — right for the
human's press, which is *stop after the current task*, but an interactive agent
part way through its work idles rather than exiting, so an agent's call would
never move anything. **A session-asked transfer ends the session once it next
goes idle**, the way an accepted `verkstead done` is seen out: the closing
words it prints after the call reach the Transcript, and then it is ended and
the mover runs.

- Works for every kind of session that can make the call (task 02), whether or
  not a driver is seeing it out on the Done signal — a grilling has no entry on
  `done`'s register, so the idle watch cannot live only there.
- **The runner does not read that ending as a step that failed to land.** A
  backlog step, inline run, instruction or investigation whose session was
  ended for a move stands down quietly — no stop, no Notice about a failed step,
  no relaunch here (stage 09's *going* already refuses a launch behind a
  request). The work carries on at the far end.
- **The rescue does not speak to a session idling behind the call**, and the
  stall sweep leaves it to the mover.
- A human's press still waits for the session to end on its own, unchanged.

## Acceptance criteria

- [ ] A stub agent that calls `verkstead transfer B`, prints a closing line and
      then idles (does not exit) is ended, moved and carried on at B; its
      closing line precedes *Transferred to B at the session's request* on the
      Timeline.
- [ ] A backlog step whose session made the call leaves no stop and no failed
      step Notice on A, and B's session carries the same step on.
- [ ] No rescue line is typed into a session idling behind the call.
