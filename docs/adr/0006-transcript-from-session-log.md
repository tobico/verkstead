# Transcript read from the agent's session log

The readable record of a session (the Transcript) is built from the session
log the agent's backend writes itself — for Claude Code, the JSONL under the
profile's `~/.claude/projects/` — not by un-rendering the terminal byte
stream. The obvious approach was parsing the PTY output back into markdown,
but that stream is a lossy rendering: wrapping, in-place redraws and truncated
panes cannot be reliably reversed, while the session log is the ground truth
of what was said and done. Verkstead passes the session id at spawn, so the
log's location is a fact rather than a guess, and tails it live on the same
cadence as the byte capture.

This couples Verkstead to a file format the backend owns. Two choices contain
that: lines are stored verbatim and parsed only at render time, so a format
change can never lose data, only defer rendering it (unrecognised entries are
shown as raw JSON rather than hidden); and the Capture — the raw PTY bytes —
remains a complete, independent record for any session that leaves no log at
all (stub agents in tests, other backends).

## Where an unrecognised entry is shown

Amended: *shown* has moved, without moving what is kept. A whole line of a type
this does not know folds into the Transcript's bookkeeping group, under the name
the log gave it, rather than standing in the conversation. What prompted it was
`atis-latch`, a type the backend added without announcing it, appearing between
two turns of a talk as a row saying only that this version had never met it —
a format change reported to the reader as though it were something said.
Nothing is hidden by the move: the group opens, and the name is what makes a new
kind findable to whoever comes looking for it.

The rule inside a turn is the opposite one, and deliberately: a block of a type
this does not know is part of what somebody said, so it stays inline as the JSON
it is. Lines that are not JSON at all, and lines that never said what type they
were, stay inline too — neither has a name to be filed under, and a silent fold
there would leave a hole in the record rather than tidy one away.
