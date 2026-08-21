# 02. Name the session at spawn

## What to build

Verkstead decides what a session is called instead of discovering it
afterwards. It generates a UUID, passes it to the agent at spawn as
`--session-id`, and records it alongside the session, so that finding the
agent's own session log later is a lookup rather than a guess.

This is the whole reason the next task can work. The alternative — computing
the slug the agent derives from its working directory — means reimplementing
a private algorithm belonging to somebody else's program, which can change
under us without warning. A UUID we chose ourselves cannot.

Nothing reads the id yet. The task is done when the id is passed, recorded,
and provably lands where the log will be found.

## Acceptance criteria

- [ ] The argv a session is spawned with carries the session id flag
- [ ] Running a real session leaves a log file named for that id under the
      Agent Profile's home, and Verkstead has the id stored to match it
- [ ] Stub agents, which know nothing of the flag, still start and still run
      to completion — the test suite passes unchanged
