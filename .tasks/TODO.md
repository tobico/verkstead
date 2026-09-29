# MCP servers

A session runs with no MCP servers today, on purpose: a Built Root has the
account's own taken out for every harness. This work is the one deliberate way
in. An **MCP server** is declared once in a new section of the settings page —
a name, a URL and headers, HTTP only — and attached to a Conversation from the
Attach button where a Brief is written, which grows into a menu: *Attach file*
first, then one entry per declared server. A pick becomes a chip beside the
attached files, and every session of that Conversation, on whichever of the
four harnesses runs it, is launched with that server configured.

Settled in the grilling of 2026-09-29, and the decisions are carried by the
tasks below rather than by a document of their own: HTTP only and static
headers only, no stdio and no OAuth; header values are secrets that never come
back to the page; a chip is a reference by name, read at each launch; servers
belong to the Conversation and freeze with its Brief; all of a server's tools
are allowed; an unreachable server never holds a launch.

## Tasks

- [x] 01: Declare a server in settings — [details](01-declare-a-server-in-settings.md)
- [ ] 02: Attach a server in the draft composer — [details](02-attach-a-server-in-the-draft-composer.md)
- [ ] 03: A Claude session runs with it — [details](03-a-claude-session-runs-with-it.md)
- [ ] 04: Headers, as secrets — [details](04-headers-as-secrets.md)
- [ ] 05: Codex, Grok and OpenCode — [details](05-codex-grok-and-opencode.md)
- [ ] 06: Try a server on save — [details](06-try-a-server-on-save.md)
- [ ] 07: The compose page — [details](07-the-compose-page.md)
