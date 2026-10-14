# 06. The server's strings the screen shows

## What to build

Read `docs/copy.md` first. Then bring into line with it every string the
server composes for the human to read on screen: the refusals the UI routes
answer with, the `why` and `trouble` lines carried in responses (file
operations, MCP probes, remote access, logins, transfers), the push
notification bodies, the render crate's own lines on the timeline (a binary
file omitted, a diff as it arrived, and the like — not the agent's words,
which are rendered as they came), and any condition or standing word the
server spells for the record.

Today a refusal is a lowercase fragment with no full stop — *"there is no
backlog on that Conversation"* — and the viewer draws it raw as the whole text
of a card. The guide's rule: a full sentence ready to show as it is, capital
and full stop, stating what is true and then the one action in a second
sentence where there is one, with nothing to do stated as the fact alone; no
*Error:* prefix, no apology, no exclamation mark. Vocabulary lowercase. A
notification body is a sentence naming the branch or device it is about.
Where a string is built from a fragment and an error's own text, the
sentence still has to read as one.

Nothing behaves differently: status codes, the shapes of responses and the
fields a string travels in are untouched. The Rust suites assert on several of
these strings, and those assertions move with them. The viewer's own strings
are the earlier tasks; where a viewer test fixture holds a server string,
update the fixture.

## Acceptance criteria

- [ ] No refusal, `why` or `trouble` line reaches the screen as a lowercase fragment; each is a full sentence that states what is true and then the one action where there is one.
- [ ] Push notification bodies and the render crate's own timeline lines read as the guide says, with vocabulary lowercase.
- [ ] The Rust and web suites pass with their assertions and fixtures updated, and no response shape or status code changed.
