# 03. Tail and store the Transcript

## What to build

While a session runs, Verkstead follows the session log the agent writes for
itself and keeps every line of it. The log is found by globbing the Agent
Profile's projects directory for the file named after the session id task 02
assigned — deliberately not by computing where the agent would have put it.

Lines are stored **verbatim**, exactly as written, and parsed only when
something asks to read them. That is what contains the coupling to a file
format somebody else owns: a change to the format can defer rendering, but it
can never lose what was said. Rows are appended keyed by the Event and a
sequence number — the same append-only shape the Capture chunks already use,
and for the same reason.

Following is plain polling on the cadence the byte relay already flushes on.
No file watching: the relay is awake on that interval anyway, and a second
mechanism would be a second thing to get wrong. Each append nudges the
viewer, using the existing contentless Nudge — an open pane's reaction is to
read everything again, so there is nothing finer to say.

A session that leaves no log at all is the ordinary case for stub agents and
any other backend. It stores nothing and complains about nothing.

## Acceptance criteria

- [ ] Rows accumulate while a real session runs, and an open pane sees them
      arrive without a reload
- [ ] A poll that catches the log mid-line never stores a torn line — the
      partial waits for the rest of it
- [ ] Lines are stored exactly as written, with nothing parsed or normalised
      on the way in
- [ ] A session with no log stores nothing, logs no error, and ends normally
- [ ] The Capture keeps recording throughout, unaffected
