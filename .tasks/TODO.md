# agent-signals-done

A session Verkstead launches is an interactive agent that idles when its work is
over, so Verkstead ends it — and until now it decided when by guessing from
outside: the work landed on the branch plus a few seconds of quiet, or a minute
of quiet alone, or two Rescues and a stop. An agent that waits on something in
the background defeats every one of those. A grilling session was ended while it
waited on an answer, and an implementation session was ended with uncommitted
work while it waited on a test run.

This work replaces the guessing with two signals the agent gives through the
CLI. `verkstead done` is the one thing that ends a session, checked against the
repository at that moment and refused where the evidence does not bear it out.
`verkstead waiting <length>` declares a background wait, and a waiting session is
active rather than Idle. The Rescue stays as the net under a forgotten signal,
and escalates to the human rather than ever stopping a session. The whole of the
decision is in `docs/adr/0018-a-session-says-it-is-done.md`, and the vocabulary
is in `CONTEXT.md` under **Done signal**, **Declared wait**, **Rescue**, **Idle**
and **Step** — both written by the grilling that settled this, and both
describing the finished work rather than the code as any one task finds it.

## Tasks

- [ ] 01: `verkstead done` ends a session that landed its work — [details](01-done-ends-a-landed-session.md)
- [ ] 02: Done is refused over uncommitted changes, and locks an open Set — [details](02-uncommitted-changes-and-open-sets.md)
- [ ] 03: Sessions that report through a commit — [details](03-commit-reported-sessions.md)
- [ ] 04: Sessions that report through their own words — [details](04-word-reported-sessions.md)
- [ ] 05: Follow-up sessions — [details](05-follow-up-sessions.md)
- [ ] 06: Done checks for the pull request — [details](06-pull-request-check.md)
- [ ] 07: The Rescue escalates and never stops a session — [details](07-rescue-escalates.md)
- [ ] 08: `verkstead waiting` declares a background wait — [details](08-declared-wait.md)
- [ ] 09: Sweep what still describes ending on quiet — [details](09-sweep.md)
