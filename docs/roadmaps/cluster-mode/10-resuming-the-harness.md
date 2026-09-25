# 10. Resuming the harness

## Goal

A transferred agent keeps its context. On arrival, where the harness has a
resume of its own — Claude and Codex first — the session is started with it
against the log the memory sync carried over, and a short note says it now
runs on B with the worktree at its new path and the new ids of any Sets it
had open, the wait on them having gone with the process. Where the harness
has none, or the log is not found, Verkstead's Resume stands as before.
Demonstrable end to end: a grilling mid-interview on A carries on from the
same question on B.

## Decisions in force

- **The harness's resume where it has one, Verkstead's as the fallback**
  ([ADR-0020](../../adr/0020-cluster-mode.md), *Transfer*). Verkstead's alone
  was rejected because the Brief asks for the session to continue.
- **The log travels with the memory sync** (stage 08), its directory
  renamed for the new worktree path; Verkstead already picks the session id,
  so finding the log is a lookup.
- **The note** names the device, the path and the Set ids; nothing beyond
  the ordinary re-prime was rejected.
- **Per harness**: Claude `--resume <id>`, Codex `resume <id>`, Grok and
  OpenCode as their lines allow — OpenCode's `--session` validates against
  its store, which the sync has to have carried.

## Proposed tasks (provisional)

1. **Resume lines per harness** — the launch line table grows a resume shape
   per harness, with the prompt carried as the first message.
   - Each harness starts against a moved log and answers the note.
2. **Log relocation** — the session's log located through the session-name
   record, its directory renamed to the new worktree's encoding.
   - Claude finds the session under the new path.
3. **The note and the Set map** — old Set ids mapped to new on the record,
   said in the note.
   - `verkstead answers <new id>` returns the answers to the old Set.
4. **The fallback** — no resume line, or no log: Verkstead's Resume, with a
   Notice saying which.

## Re-verify at start

- Stage 09 landed: the move and Verkstead's Resume on arrival.
- The launch line is still `Agents::argv` and `line` in
  `crates/server/src/sessions.rs`; session ids in
  `crates/store/src/session_names.rs`; log shapes in
  `crates/server/src/transcript.rs`.
- Each harness's resume flag and whether it accepts an initial prompt — check
  the installed versions rather than the docs.
