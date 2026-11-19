# 03. Codex's plugin, manifest and component

## What to build

`plugins/codex/`: a `plugin.yaml`, the mark (`harness/marks/codex.svg` and its
lobehub licence notice), the Guide's store-and-nudge section as the manifest's
`ask.guide` markdown, and a Rust crate in the `plugins/` workspace compiled to
`codex.wasm` (see `crates/server/build.rs` and `docs/development.md`'s
*Built-in plugins* for how a built-in's component is built and embedded). Its
uuid is Codex's built-in uuid from `store::BUILT_IN`, so stored `codex`
Profiles read as it. Everything is carried over from the in-process
implementation (`harness/codex.rs`) and not re-decided — ADR-0011's *As built
for Codex* and the agent-backends roadmap's stage 03 say why each piece is what
it is:

- **Manifest**: binary `codex`, name `Codex`; line `-m {model}`, prompt
  positional, no session on the line, resume the subcommand `resume` with no
  `{session}` and `also` `-c tui.resume_cwd="current"` (task 02); tail
  `--dangerously-bypass-approvals-and-sandbox`, `--no-alt-screen`,
  `-c cli_auth_credentials_store="file"`, and the Worktree trust as the whole
  table `-c projects={"{worktree}"={trust_level="trusted"}}` (the table, not a
  dotted key — codex splits keys on every dot); one account path, a directory
  labelled *Home directory*, mounted at `.codex`, credentials `auth.json`,
  configuration `config.toml`, memory `sessions` (synced by Worktree) and
  `memories` (whole); instructions `AGENTS.md`; the at-work signature; the
  usage-limit phrase `you've hit your usage limit` (ASCII apostrophe); no
  signed-out phrase and no login; channel store-and-nudge; tools
  `mcp__<server>__<tool>`; models `gpt-5-codex` (*GPT-5 Codex*) as the default.
  Check every placeholder and override against what the in-process `line()`
  produces, with and without a Worktree.
- **The at-work signature, re-measured** (Set 1836, Q7). `esc to interrupt` was
  read off codex 0.149.0 and is no longer a literal in the 0.164.0 binary on this
  machine. Measure it: a throwaway HOME whose `config.toml` points
  `model_provider` at a local stub Responses-API server that answers slowly, so
  a turn is in progress with no account; read what the screen has while working
  and not while idle. Write what is measured into the manifest, with the release
  it was read off in a comment.
- **Component, `parse-transcript`** (in task 01's shape): with no id, walk
  `/account/sessions/` (filed by date `YYYY/MM/DD`, `.jsonl.zst` and anything
  not `rollout-*.jsonl` skipped) for a rollout written after `launched` whose
  opening `session_meta` names the Worktree; answer the id it gives itself
  (`session_id`, as the server's `declares` reads it). With an id, find the
  rollout by it. Then the line parser: the render crate's rollout drawing
  (`event_msg` items → said / tool call / result) and its `ROLLOUT` list as
  notices, a kind nothing knows as unknown under its own name. The render
  crate's own reader stays for stored history (Q3a) — this is a copy that
  becomes Codex's, not a move.
- **Component, `write-configuration`**: `config.toml` carrying only the
  account's `model_provider` and `model_providers` and the attached servers
  under `mcp_servers` with headers as `http_headers` — what `toml_carrying`
  with `CODEX_CARRIED`/`CODEX_HEADERS` does today, byte for byte.

The plugin is embedded but not yet stood up as the harness (task 04 does that),
so it is proven here by tests that load its component.

## Acceptance criteria

- [ ] `verkstead plugin check plugins/codex` accepts it, and a Profile form drawn from its manifest has one path field labelled as today's
- [ ] The render suite's Codex rollout fixtures, through `parse-transcript`, give the same said/tool/result/notice/unknown kinds the render crate draws from them
- [ ] For each account configuration the sandbox suite's Codex tests use, with and without attached servers, `write-configuration` answers the same `config.toml` bytes as the in-process `configured`
- [ ] The launch line built from the manifest equals the in-process `line()` for a launch and a resume, with and without a Worktree; the re-measured signature is in the manifest with the release it was read off
