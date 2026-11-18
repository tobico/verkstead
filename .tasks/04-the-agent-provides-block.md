# 04. The `agent` provides-block and the WIT

## What to build

Write the grammar of a `provides:` entry of category `agent` into
`docs/plugins.md`, which is the one place each Category's grammar is written.
Read it in the manifest reader, refusing by field as everything else there is.
Each piece below is **data** (ADR-0030, *Agent*; Set 1764):

- the binary's name, the display name, and the mark. The plugin's top-level
  `mark` may serve.
- the launch line, written with placeholders the host fills: the model flag,
  the prompt flag or positional, whether the session id is named on the line,
  the resume shape (a flag or a subcommand), the tail flags, and a session-only
  settings JSON
- the account as a **list of labelled paths**, each with its key, its label,
  whether it is a directory or a file, where it is mounted inside the sandbox,
  and its allowlist, credential files and memory directories
- idle detection, as a screen signature or byte silence
- the usage-limit and signed-out phrases
- the ask channel (blocking or store-and-nudge), and the *Running the ask*
  section of the Guide as markdown
- the model list, each with an id and a name, with one `default` and optional
  per-role defaults keyed by the three known role names `grilling`,
  `implementation` and `review` (Set 1786, Q4)
- the MCP tool-name syntax the prompt uses
- an install-instructions link and per-platform install commands
- whether it logs in, and the login line where it does
- **session env vars, the instructions file name, and the paths kept out of the
  worktree** (Set 1786, Q5)

Every piece can be overridden per platform under `linux`, `macos` and `windows`,
over the common declaration, replacing whole as the top-level overrides already
do.

Also write the WIT `agent` world in `crates/server/wit/plugin.wit`, exporting
`parse-transcript`, `write-configuration` and `login`, and generate the host
bindings. Shape the exports from what tasks 01 and 02 settled about the trait:

- `parse-transcript` takes the account's root and the session id, and answers
  with said, tool call, result, system notice and unknown.
- `write-configuration` takes the configuration as found plus the attached
  servers, and answers with the root's files and the session-only settings.
- `login` takes what the harness wrote, and answers with an address to show, a
  code to type, words to say, or done.

A manifest providing `agent` must name a component.

## Acceptance criteria

- [ ] `verkstead plugin check` accepts a complete agent manifest. It refuses one
      missing a required piece, or holding an unknown key, naming the field
      (`provides[0].account[1].label`), on every platform where it fails.
- [ ] A per-platform block replaces its key on that platform and nowhere else.
- [ ] The `agent` world with the three exports is in the repository, with host
      bindings generated and compiling. A component that does not export them
      is refused for an agent provision.
