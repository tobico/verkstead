# 10. The design Process as the acceptance test

## What to build

The human's design Process, written as a fixture plugin that names nothing but
existing Activities and a Skill pack of its own — the proof that a Process
needs no code:

- `rounds` — Follow-up rounds under the plugin's own skill, with pictures, in a
  Conversation with an MCP server attached (attached by the human, as any MCP
  server is, through the Attach menu);
- `export` — One session, exporting what the rounds made;
- `wrapping` — the Wrap-up.

It depends on the *Activities* and *Verkstead* plugins, declares its roles and
its steer marks, and passes `verkstead plugin check`. If writing it shows the
grammar lacking something, fix the grammar — not the fixture — and say so in
`docs/plugins.md`.

## Acceptance criteria

- [ ] The fixture plugin loads, passes `verkstead plugin check`, and contains no
      wasm component of its own
- [ ] In the suite, a Conversation on it with an MCP server attached runs its
      rounds under the plugin's skill, then the export, then the wrap-up, to Done
- [ ] Nothing outside the fixture plugin was added to make it run
