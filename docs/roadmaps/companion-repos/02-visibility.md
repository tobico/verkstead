# 02. Visibility

## Goal

Work in companion repos is visible where the main repo's already is: commits
on a read-write companion's branch land on the Timeline as they happen, labeled
with the repo they belong to, and the diff under a Question Set shows every
repo's uncommitted changes — the main repo's included, now composed by the
server.

## Decisions in force

- **A commit sweep per read-write companion, per session.** The sweep already
  watches one branch per session from a single call site in the session relay
  (`crates/server/src/sessions.rs`); companions each get a watcher of the same
  shape, stopped and awaited with the session so the final commit is caught.
  Read-only companions have nothing to sweep.
- **Commit Events carry the repo's name.** With several repos feeding one
  Timeline, an unlabeled commit card is ambiguous. Settled Q10.
- **Set diffs are composed by the server, per repo, main repo included.** The
  CLI's diff enrichment is retired: the server knows the Conversation a Set
  arrives from (the scoped `VERKSTEAD_SERVER`) and reads every worktree on the
  host, so all derivation lives in one place — the human's explicit
  consistency call on Q16. This *strengthens* ADR-0001's determinism-over-trust
  rather than bending it: the server trusts its own reads over anything sent.
  The CLI keeps deriving project and branch.
- **Companion diffs are uncommitted-only**, like the main one — committed work
  is already on the Timeline as Events. Rendered as labeled per-repo diff
  blocks under the Set.

## Proposed tasks (provisional)

1. **Per-companion commit sweeps** — spawn one watcher per read-write
   companion beside the main one; commit Events and their store rows carry the
   repo; Timeline and details pane draw the label (main repo's commits may
   stay unlabeled — the label earns its place when repos mix).
2. **Server-composed Set diffs** — on Set arrival, read uncommitted changes
   from the main and each read-write companion worktree; retire the CLI's
   `diff` enrichment; store and render per-repo diff blocks, labeled.
   - A clean worktree contributes no block; all clean means no diff, as today.

## Re-verify at start

- Assumes stage 01 landed: companion config, worktrees and store relations
  exist.
- The single sweep call site and its `(stop, handle)` pair in
  `crates/server/src/sessions.rs` (around the session relay) — still exactly
  one watcher.
- The CLI's enrichment in `crates/cli/src/repo.rs` and where `set.diff` is
  attached in `crates/cli/src/ask.rs`; where the server stores a Set's diff
  and how the viewer renders it.
- How commit Events are stored and rendered (`crates/server/src/commits.rs`,
  render types) — whether a repo column fits the existing rows or needs a new
  one.
