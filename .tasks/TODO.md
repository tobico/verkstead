# Claude's built root

A Claude Code session, on Linux, a Mac and Windows alike, runs in a `.claude`
that is Verkstead's own rather than in the human's whole account. The root
holds the account's credentials, linked so that a login from inside writes
through. It holds the Repo's and the Worktree's `projects/` entries, so memory
and transcripts still land in the account. It holds a `settings.json` Verkstead
wrote, so the session never stops at the bypass-permissions consent. Nothing
else of the human's is in it: no plugins, hooks, global `CLAUDE.md`, history,
MCP servers or other repositories' transcripts.

This closes findings 9, 9d, 9e, 9f, 10 and 12 from the Windows reports, and the
same inheritance on this repository's own Linux sessions. A fresh Windows
install with a plain npm `claude` presses Start work and the session works.

Roadmap stage: [01: Claude's built root](docs/roadmaps/built-roots/01-claude-root.md)

## Tasks

- [x] 01: The built root on Linux and a Mac — [details](01-root-on-linux-and-mac.md)
- [x] 02: The built root on Windows, with the grant narrowed — [details](02-root-on-windows.md)
- [x] 03: The written `settings.json` — [details](03-written-settings.md)
- [x] 04: `.claude.json` copied, pre-seeded and merged back — [details](04-claude-json-copy-and-merge.md)
- [ ] 05: The docs — [details](05-docs.md)
