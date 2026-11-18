# 01. The trait, launch half

## What to build

Introduce one trait in the server through which every harness is reached
(ADR-0030, *Agent*; stage brief, *One trait, six implementations*). Give it five
in-process implementations — Claude, Codex, Grok Build, OpenCode and the
Antigravity CLI — whose behaviour is **byte-for-byte what it is today**. Then
replace every branch on `AgentType` that decides *how a session is launched and
watched* with a call to it.

This task covers these operations:

- the binary's name (`PROGRAMS` included)
- the launch line: the model flag, the prompt flag or positional, whether the
  session id is named, the resume shape (flag or subcommand), the tail flags,
  and agy's session-only `--settings` JSON. This includes the read that decides
  whether a resume is possible.
- the idle screen signature or byte silence, and what is judged from it
- the usage-limit and signed-out phrases
- the ask channel, blocking or store-and-nudge. `AgentType::channel` moves off
  the store type.
- the MCP tool-name syntax in `skills.rs`
- the display name (`transfers.rs`)
- the session env vars (Claude's `DISABLE_AUTOUPDATER`, OpenCode's database and
  bash timeout, agy's Windows file store)
- the paths kept out of the worktree (`.gemini/`)
- the desktop-app detection

Choose the trait's shape so that a later wasm-backed implementation can answer
the same calls from a manifest (task 05). Where a piece is data in the manifest
(see task 04), prefer a method that returns the data over one that acts on it.
**Desktop-app detection stays in Claude's Rust implementation** (Set 1786, Q5):
stage 06 settles it, and it is not manifest data.

**The name `Harness` is already taken twice, privately**: an `enum Harness` in
`sandbox/root.rs` and a `struct Harness` (the login child) in `logins.rs`.
Rename them, or pick another name for the trait. Whatever is visible to the
human follows CONTEXT.md and docs/copy.md.

The sessions suite's `Agents::running` stub override still has to work. It
replaces the binary with an argv and keeps the type's own line. Route it through
the trait, or keep it beside the trait.

Leave the account half (task 02) and the id's representation (task 03) alone.
`AgentType` stays an enum for now, and the trait is looked up from it.

## Acceptance criteria

- [ ] One trait and five implementations exist. None of the operations listed
      above branches on `AgentType` or `Account` in non-test server or CLI code.
- [ ] The sessions, limits, signouts and skills suites pass unchanged, with no
      test edited to pass.
- [ ] The private `Harness` names no longer clash with the trait.
