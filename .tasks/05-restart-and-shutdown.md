# 05. Restart and shutdown

## What to build

A shutdown or restart of Verkstead carries a mid-turn session on. No session
survives the process, and `resume::at_startup` presses Resume for every
Conversation the last server was driving — so the decision is read at launch from
the record of how the last session ended, since nothing ran at the restart (or a
crash, or a kill) to write a row.

`at_startup` reaches `recompute`, so the detection from tasks 01–02 applies: a
Conversation whose newest session was cut off mid-turn (a grilling always; an
Implementing one by its task state; a crash by its `session_endings` row) is
carried on, and one left cleanly between tasks starts fresh. A stop somebody
decided on is still left for the human, exactly as `at_startup` leaves it today.

**Careful on this machine.** This Conversation runs on the Verkstead the change is
about — do not restart, stop or replace `verkstead.service` on this annex to test a
restart; it would end the session doing the work. Test shutdown and restart
against a **second Verkstead** with its own Data Directory and ports, as
docs/development.md describes.

## Acceptance criteria

- [ ] A restart over a running mid-turn session carries it on with the harness's own resume.
- [ ] A restart over a Conversation left cleanly between tasks stays Verkstead's own Resume.
- [ ] A stop the human or Verkstead decided on is still left for a press across the restart.
- [ ] The restart tests run against a second Verkstead, never this annex's service.
