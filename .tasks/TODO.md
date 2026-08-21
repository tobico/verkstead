# Transcript

A session's details pane shows the conversation rather than the bytes: the
agent's prose rendered as markdown, tool calls collapsed to one-line
summaries, human turns, and thinking collapsed — live while the session runs.
The content comes from the agent's own session log, never from un-rendering
the terminal stream, because that stream is a lossy rendering and the log is
the ground truth of what was said and done ([ADR
0006](../docs/adr/0006-transcript-from-session-log.md)).

The word *transcript* currently means bytes throughout the code, and the
glossary now gives it to the readable record. So the rename comes first and
goes all the way down: what was a transcript becomes a **Capture**, and the
name is free for what these tasks build. Sessions that leave no session log —
the stub agents the test suite runs on, any other backend — keep showing the
Capture exactly as today, which is what makes the fallback a real record
rather than an apology.

Roadmap stage: [01: The Transcript](../docs/roadmaps/session-output/01-transcript.md)

## Tasks

- [x] 01: Rename the byte capture — [details](01-rename-the-capture.md)
- [x] 02: Name the session at spawn — [details](02-name-the-session.md)
- [x] 03: Tail and store the Transcript — [details](03-tail-the-session-log.md)
- [x] 04: Render the conversation — [details](04-render-the-conversation.md)
- [ ] 05: Switch summaries and evidence — [details](05-summaries-and-evidence.md)
