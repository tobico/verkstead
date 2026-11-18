# 04. Codex across

## Goal

Codex is the first built-in harness to become what a user writes: a directory
with a `plugin.yaml` and a wasm component, embedded in the binary and listed
under **Built-in** on the Plugins page, with the in-process Rust implementation
of stage 03's trait deleted. A Codex Profile is made, paired and launched exactly
as before; the session is nudged over its store-and-nudge channel; its rollout
is drawn on the Timeline; a Conversation resumes it by the id it chose for
itself; the declared MCP servers reach it through its `config.toml`. The suite
that proved Codex when it landed passes unchanged against the component, which
is what proves the boundary carries a real harness and not only the stub of
stage 03.

## Decisions in force

- **[ADR-0030](../../adr/0030-plugins.md), *The shape of a plugin* and
  *Agent***: every built-in is the artefact a user writes. Set 1760 settled the
  built-in harnesses as in-process Rust behind the trait; Set 1769 (Q41)
  revised that, after GitHub and the Activities had moved across to prove the
  boundary — **yes, one at a time as stages, Claude last**. This is the first.
- **Why Codex first.** It has no login Verkstead runs, so the `login` export is
  not exercised; its record is a plain JSONL rollout, so `parse-transcript` is a
  line parser and nothing more; and its account is one home directory, the
  shape every harness after Claude keeps. It is the smallest crossing that still
  exercises two of the three exports, and what it teaches about the boundary is
  learned before Antigravity's database and Claude's daily use are at stake.
- **Everything measured about Codex carries over as the component's
  behaviour**, and is not re-decided here: ADR-0011's *As built for Codex*
  paragraphs and [the agent-backends roadmap's stage 03](../agent-backends/03-codex.md)
  — the `~/.codex` home, `config.toml` as the written configuration, the `-c`
  overrides that mark the Worktree trusted and point at the root's credentials,
  `resume` as a subcommand rather than a flag, the `CODEX_AT_WORK` screen
  signature, the usage-limit phrase, and the session name Codex chooses for
  itself and Verkstead records in `session_names` to resume by.
- **The rollout parser moves into the component.** The render crate's `ROLLOUT`
  list — `response_item`, `session_meta`, `turn_context`, `world_state`,
  `compacted` and the rest that are the session's record of itself rather than
  the conversation — becomes the component's own knowledge, and what it answers
  with is the Transcript pane's kinds. A kind it has not heard of still folds
  away under its own name (ADR-0006); that rule is the host's and does not move.
- **Store-and-nudge is declared, not implemented.** The channel is a manifest
  word; the nudge that types the Answers into the terminal is the host's
  (`nudging.rs`), and the Guide's store-and-nudge section is the manifest's
  markdown. Nothing of the ask machinery crosses.
- **Embedded, and listed as Built-in.** The component is built in CI and shipped
  inside the binary beside the viewer and the skills; the Plugins page lists it
  with no Remove and no Update, and its tick is the one switch for Codex.
  Disabling it follows stage 03's rule.
- **The in-process implementation goes.** Keeping both would be two Codexes to
  keep true. The suite is the proof that nothing was lost, so the suite is not
  edited to pass: a test that fails against the component is a difference in the
  component.

## Proposed tasks (provisional)

1. **The manifest** — Codex's `plugin.yaml` under the embedded plugins: binary
   `codex`, the launch line with `resume` as a subcommand, the `-c` tail, one
   declared home, its allowlist and credential file, the signature, the phrase,
   the store-and-nudge channel and its Guide section, the model list. AC:
   `verkstead plugin check` accepts it; a Profile made from it draws one path
   field.
2. **The component** — `parse-transcript` over the rollout (discovery by the
   recorded session name under `sessions/`, then the line parser) and
   `write-configuration` producing `config.toml` with attached MCP servers; no
   `login`. Built from a Rust crate under the repository, compiled to a
   component in CI. AC: the rollout fixtures the render suite holds parse to the
   same kinds as before; a Conversation with a declared server gets it in the
   root's `config.toml`.
3. **Embedding and the Built-in row** — the plugin shipped in the binary, loaded
   at startup before any user plugin, listed under Built-in. AC: a fresh data
   directory lists Codex; the row has a tick and nothing else.
4. **The crossing** — the in-process Codex implementation deleted, every Codex
   path in the sessions, transcript and nudging suites run against the
   component. AC: the suites pass unchanged; `grep -i codex` in non-test server
   code finds only the embedded plugin's directory and the migration that reads
   old Profiles.
5. **Memory and mirroring** — the memory roots the peer and mirroring code read
   per harness come from the manifest's declared memory directories. AC: a Codex
   Profile's memory syncs to a device it is not bound on as before.

## Re-verify at start

- Stage 03 landed: the trait exists with the five in-process implementations and
  the wasm-backed sixth, the manifest grammar for an Agent is settled, and a
  stub harness plugin runs end to end.
- Codex's record is still a JSONL rollout under `sessions/` named by the id it
  chooses; a Codex release since may have changed the kinds it writes, in which
  case the `ROLLOUT` list is out of date before it moves.
- `session_names.rs` is still where the chosen id is recorded and read back for
  a resume.
- The `CODEX_AT_WORK` signature still matches the current Codex release's
  at-work label; re-measure in a throwaway HOME before writing it into the
  manifest.
- CI can build a wasm component for every release platform's binary to embed:
  the musl Linux build, the lipo-merged Mac binary and the Windows one all carry
  the same component bytes, so one `wasm32-wasip2` build per release is enough —
  check the release workflow has a job for it since stage 01.
