# The Process engine

A plugin may provide a **Process**: declared states over Activities, with
transitions keyed on the Outcomes those Activities end in, and Verkstead runs a
Conversation by reading its Process's manifest rather than by matching on a
shared enum. The five built-in Processes — Develop, Investigate, Review, Tinker
and Fix Merge Issues — are written as data in five bundled plugins, each with a
tick on the Plugins page, and every Conversation already in the database reads
its state against the plugin its Process names.

The shared `Lifecycle` and the store's `Process` enum go; the viewer draws
picker rows, descriptions, state words and Steer rows from what the manifests
declare; and the human's design Process — rounds with pictures under an
attached MCP server, an export session, the wrap-up — is written as a plugin
naming nothing but existing Activities and a Skill pack of its own, which is
the stage's acceptance test.

Roadmap stage: [11: The Process engine](docs/roadmaps/plugins/11-the-process-engine.md)

## Tasks

- [x] 01: The `process` provides-block — [details](01-the-process-block.md)
- [x] 02: The engine, end to end on a fixture Process — [details](02-the-engine.md)
- [x] 03: Tinker, Investigate, Review and Fix Merge Issues as data — [details](03-four-processes-as-data.md)
- [x] 04: Steers land on declared states — [details](04-steers-on-declared-states.md)
- [ ] 05: Develop as data — [details](05-develop-as-data.md)
- [ ] 06: Roles from the manifest — [details](06-roles-from-the-manifest.md)
- [ ] 07: The viewer from the manifest — [details](07-the-viewer-from-the-manifest.md)
- [ ] 08: The store's `Process` enum goes — [details](08-the-process-enum-goes.md)
- [ ] 09: `Lifecycle` goes — [details](09-lifecycle-goes.md)
- [ ] 10: The design Process as the acceptance test — [details](10-the-design-process.md)
