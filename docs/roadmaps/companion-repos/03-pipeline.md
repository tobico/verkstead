# 03. Pipeline

## Goal

Wrap-up covers every touched companion: the finish session opens a pull
request per companion holding commits, the server discovers and records each
one, checks and comments are watched per PR, one review session reviews the
whole of it, and Done waits for every PR to settle. A touched companion the
finish left without a PR stops the run with a Notice naming it.

## Decisions in force

- **Touched means commits beyond the base at finish time.** Read-only
  companions and read-write ones with no commits are ignored by wrap-up
  entirely — no PR expected, nothing waited on.
- **The finish session opens the PRs, the server discovers them** — the
  existing split ("the push and the pull request are the session's; what is
  Verkstead's is knowing that it happened"). The finish-sequence text in the
  skills extends to: for each touched companion, follow *that* repository's own
  `git-workflow.md` review process.
- **A touched companion without a PR is a deliberate stop** with a Notice
  naming the companion — the same shape as today's missing main PR. Settled
  Q6 over carrying on with what was found.
- **One PR per repo per Conversation, several per Conversation.** The
  `pull_requests` table's `UNIQUE (conversation_id)` becomes per-repo; every
  reader that assumes at most one (`store::pull_request`'s `Option`, the
  pinned-event `find_map`, checks and comments loading "the" PR) learns
  plurality. PR Events and the pinned block name their repo.
- **Checks and comments watchers per PR**, bookkeeping keyed by PR: two fix
  attempts per check *per PR*; addressed comments per PR. The addressing and
  responding sessions get feedback naming the repo and PR — their sandboxes
  already hold every worktree. Sessions still run one at a time per
  Conversation, so two red PRs queue rather than collide.
- **One review session, one Question Set, across all PRs** — the work was
  conceived as one Conversation and reads best whole (settled Q7 over per-PR
  review sessions). Review settles once; comments-gating waits on that one
  review as today. The reviewing prompt lists every PR.
- **Done waits for every watched PR** to settle checks and comments, plus the
  one review. The merge is still not waited on, for the same reason as ever —
  stages stack on unmerged predecessors.

## Proposed tasks (provisional)

1. **PR plurality in the store** — reshape `pull_requests` to one row per
   (conversation, repo), thread repo identity through the Event, the pinned
   block and the details pane.
2. **Discovery per touched companion** — after the finish, ask each read-write
   companion repo for a PR on its branch; record what is found; stop with the
   naming Notice when a touched companion has none.
3. **Checks per PR** — a watcher per recorded PR, fix attempts keyed by PR and
   check, addressing feedback naming repo and PR.
4. **Comments per PR** — same shape; for-the-review capture and
   addressed-comment bookkeeping per PR; responding feedback naming the repo.
5. **Review across PRs and the skill text** — the reviewing prompt lists every
   PR; finish sequences in the next-task, implementing and staging skills
   extend to touched companions.
6. **Done** — settling waits on every PR's checks and comments plus the one
   review; the wrap-up settled bookkeeping gains the per-PR dimension.

## Re-verify at start

- Assumes stage 01 landed (02 is reorderable with this one — if 02 has not
  landed, Set diffs and commit labels are absent but nothing here depends on
  them).
- The four-watcher shape in `crates/server/src/wrapping.rs::watching`, and
  `WAITED_ON` in `crates/store/src/wrap_up.rs` — still three settles per
  Conversation.
- `UNIQUE (conversation_id)` still on `pull_requests`
  (`crates/store/src/pull_requests.rs`), and the record-once branch inside
  `record_pull_request`.
- The finish text in `crates/server/skills/next-task/SKILL.md` (and
  implementing, staging) — where the companion sentence lands.
- The review split-out path (`store::implement_again`, second wrap on the same
  PR row) — reconcile with plural rows.
