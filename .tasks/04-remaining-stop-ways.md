# 04. The remaining stop-ways

## What to build

Carry the detection across every remaining in-scope way a session is cut off,
including the ones whose newest session left an ending record rather than a live
kill:

- **A sign-out stop** (`signouts`) — the case that prompted the Brief. Its resume
  comes through `signouts::logged_in` / `carry_on` → `resume(Resuming::LoggedIn)`,
  which already funnels to `recompute`.
- **A late sign-out**, where the login was replaced since (`signouts::late`,
  `Decided::Nobody`) — resumed by `signouts::carrying_on`.
- **A usage-window stop** (`limits`, `Decided::OutOfWindow`).
- **A stall stop over a crashed session** (`stalls`, `Decided::Circumstance`).
- **A harness that exited or crashed mid-turn**, which the relay records as
  `Ended::Badly` with a `session_endings` row.

Where a session left a `session_endings` row (a crash or bad exit), read it as
part of the mid-turn signal; where the stop ended a live session (sign-out,
force stop, window), that is the live-kill signal from task 01. A **stall with
nothing running**, or a server that runs no agents, has no mid-turn session and
stays Verkstead's own Resume.

A sign-out cut-off here exercises the task-03 short-circuit, which is the full
incident end to end: a sign-out mid-answer carried on rather than diverted.

## Acceptance criteria

- [ ] Sign-out, late sign-out, usage-window, stall-over-a-crash, and a harness crash mid-turn are each carried on with the harness's own resume.
- [ ] A stall with nothing running, and a server with no agents, stay Verkstead's own Resume.
- [ ] The full sign-out incident — cut off mid-answer, carried on, acting on the answers rather than opening a pull request — passes as a test.
- [ ] Each occasion has its own test.
