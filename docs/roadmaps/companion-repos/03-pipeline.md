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
   - The reshape is a **rebuild migration** in `crates/store/src/migrations.rs`
     rather than a new declaration: `UNIQUE (conversation_id)` is on a table
     that already exists, SQLite will not drop a constraint by `ALTER TABLE`,
     and the store's `CREATE TABLE IF NOT EXISTS` does nothing at all to a
     database that has the old shape. Without it the stage passes on a fresh
     database and refuses the second pull request on a real one.
   - New table, rows copied across, old one dropped, new one renamed — the
     module has read old rows, written new ones and dropped a table before,
     but has never rebuilt one. Safe to run twice, like the rest of it.
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
   - The skill *bodies* extend too, not only the prompts they are wrapped in:
     `reviewing`, `addressing` and `responding` are each written for one
     branch and one pull request throughout — `gh pr diff`, `gh pr checks`,
     one push at the end, and "do not touch any other branch".
   - A session is chdir'd into the **main** worktree and `gh` reads its
     repository from where it runs, so a session sent at a companion's pull
     request is told which worktree to work in — or it asks the wrong
     repository how its checks are getting on.
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
- What `crates/store/src/migrations.rs` has learned to do by then — still
  row rewrites and a dropped table, or a rebuild to follow.
- The finish text in `crates/server/skills/next-task/SKILL.md` (and
  implementing, staging) — where the companion sentence lands.
- The single-PR language still in `crates/server/skills/reviewing`,
  `addressing` and `responding` — and whether the sandbox still chdirs into
  the main worktree alone (`--chdir` in `crates/server/src/sandbox.rs`).
- The review split-out path (`store::implement_again`, second wrap on the same
  PR row) — reconcile with plural rows.
