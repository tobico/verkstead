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
