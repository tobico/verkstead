# 05. A plugin harness launches

## What to build

Build the sixth implementation of the trait: one constructed from a loaded
plugin's agent provision and its wasm component. A third-party plugin is only
ever this one. With it in place, an enabled Agent plugin's harness joins the
server's harness list from task 03, under its uuid, its name and its mark.

- Fill the placeholders of the declared launch line: model, prompt, session id
  and resume.
- Mount the declared paths and apply the allowlist and credential files.
- Idle by the declared signature or byte silence, and use the declared phrases.
- Ask over the declared channel, blocking or store-and-nudge, with the nudge
  staying the host's own.
- Build the Built Root by calling `write-configuration` with the account's
  configuration as found, with its own servers already taken out, and the MCP
  servers attached to the Conversation (ADR-0021).

Make a **stub harness plugin** for the suites: a manifest and a component
around a script that behaves like a harness and writes a JSONL record. Build it
the way `plugins/` builds components for `wasm32-wasip2`, but install it only
in tests, never as a built-in. It may replace `Agents::running` where that
reads better, but need not.

## Acceptance criteria

- [ ] The stub plugin, installed from a directory, appears on the harness list.
      A Profile of it is saved on the Profile form, with one field per declared
      path, and bound. It is paired on the composer under its own mark.
- [ ] A session under it launches with the declared line. It idles as declared,
      by signature and, in a second manifest, by silence. It blocks, or is
      stored and nudged, as its channel says.
- [ ] A Conversation with a declared MCP server attached has that server in the
      configuration `write-configuration` wrote into the Built Root. The
      account's own servers do not reach the export.
