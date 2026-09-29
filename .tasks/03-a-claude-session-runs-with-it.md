# 03. A Claude session runs with it

## What to build

A session launched on Claude Code for a Conversation with MCP servers attached
runs with those servers configured, and is told so.

**When the Built Root is made, the attached servers are written into it.** The
root's copy of `.claude.json` has the account's own `mcpServers` taken out, and
that stays exactly as it is; what goes in afterwards is Verkstead's own, one
entry per attached server that is still declared:

```json
{"mcpServers": {"<name>": {"type": "http", "url": "<url>"}}}
```

`type` is required — Claude Code refuses a `url` without it. Nothing of this
may reach the account's file when the session ends: the merge back already
neither writes nor removes `mcpServers`, and a test should hold it to that with
a Verkstead-written server in the copy.

**Read at launch, by name.** Which servers the Conversation has comes from its
record and what each one is comes from settings at that moment, like everything
else a root is built from — so an edit reaches the next session and a running
one keeps what it started with. **A server attached but since deleted is
skipped**, silently as far as the launch goes.

**Every session of the Conversation**, not the grilling alone: there is one
place every session is launched from, and that is where this belongs, so a
session kind added later cannot forget it.

**The prompt names them.** Beside the section that lists attached files, a
short one naming each attached server and how its tools are named for this
harness — `mcp__<name>__<tool>` on Claude. No section at all where there are
none.

**Nothing is checked first.** A server that cannot be reached does not hold the
launch; the harness reports it as failed and the session carries on without
it. All of a server's tools are allowed — sessions already run with permission
prompts off, and there is no per-tool filter.

Only Claude in this task. The other three harnesses are task 05, and until then
a session on one of them launches as it does today.

## Acceptance criteria

- [ ] A Claude session's root holds each attached, declared server under
      `mcpServers` with `type: http` and its URL, and none of the account's own.
- [ ] A Conversation with nothing attached builds the root it built before.
- [ ] A server attached and then deleted from settings is left out, and the
      session launches.
- [ ] After a session ends, the account's `.claude.json` holds no server
      Verkstead wrote and has lost none of its own.
- [ ] A URL edited in settings between two sessions of one Conversation is what
      the second is given.
- [ ] The prompt of every session kind names the attached servers, and says
      nothing about servers where there are none.
