# 05. The restart

## What to build

Where a Submit applies a companion or a server, and a session is waiting on
that very Set, the agent is carried on in the wider sandbox. Only what changes
the sandbox restarts: a Companion changes what is bound and an MCP server
changes what the Built Root is written with. A device tick, a file, or a Set
with nothing pending restarts nothing. Only the session waiting on this Set is
touched: a Deferred Ask answered with additions, a Set whose asking session has
gone, and any other running session of the Conversation are left alone, the
additions reaching the next launch.

The order is the Steer's. Every question from tasks 03 and 04 is asked first,
so a refusal ends nothing. Then the waiting session is ended where it stands,
before the Response is stored, with a stop written so Verkstead reads the
ending as deliberate the way a Steer's interrupt is, and the session's wait in
`verkstead ask` never prints the Answers. The Response is stored and the
Conversation widened, the checkouts are made, and a launch follows.

The launch is the Carried Conversation's carrying-on, on this device: the
harness's own resume, primed with a note saying the sandbox changed, naming
what was added, and giving the Set's title with its `verkstead answers` line so
the agent reads the Answers once and carries on. Where the harness cannot
resume, for any of the five reasons ADR-0020 names or because the harness
refused, Verkstead's own Resume stands, re-primed from the record. The Timeline
says which it was and why, as an arrival says it, and the line under the Set's
Event says the session was carried on.

Read ADR-0028, ADR-0020's Transfer, the CONTEXT.md entries for Carried
Conversation, Stopped, Resume, Steer and Deferred Ask, and the server's
carrying and continuation code before starting; what this borrows is that
machinery, not the occasion, and a Transfer's own carrying-on must be as it was.

## Acceptance criteria

- [ ] A Submit adding a companion or server while a session waits on that Set ends that session before the Response is stored, widens the Conversation, makes the checkouts, and launches; the ended wait prints nothing and the resumed agent reads the Answers once through `verkstead answers`.
- [ ] The launch resumes the harness with the note, and where it cannot, Verkstead's Resume runs re-primed from the record; the Timeline says which harness resumed which session or why it did not.
- [ ] A device tick, a file, a Deferred Ask, a Set whose session has gone, and additions on a Set another session is not waiting on restart nothing; no running session other than the one waiting on the Set is ended.
- [ ] A refusal from tasks 03 or 04 ends nothing.
- [ ] A Transfer's carrying-on behaves exactly as before, and the sessions suite is run at CI's default threads.
