# 07. A stack of pull requests

## Goal

A **Fix Merge Issues** Conversation takes a whole stack. At Start the server
walks the chain on GitHub from the pull request it was pointed at, both ways —
a base that is another open pull request's head, a head that is another's base —
records every pull request it found on the Conversation, and says on the
Timeline what the stack is. The narrowed wrap-up then waits on Mergeable and the
checks for every one of them, and a conflict anywhere in the stack dispatches
one `addressing` session told the whole ordered list from the bottom, which
syncs the stack with `gh stack sync` and resolves what that backs out on, runs
the repository's tests and pushes every branch. A lone pull request goes on
being what stage 06 made it: one session, the configured resolution strategy,
goes counted per pull request.

## Decisions in force

- **[ADR-0020](../../adr/0020-a-conversation-has-a-process.md), *Fix Merge
  Issues***: the stack walked both ways, one session for the stack, `gh stack
  sync` for a stack whatever the configured strategy says — a stack being
  gh-stack's, and a merge into each branch of one being what the strategy's own
  documentation warns against. Walking only downward, and leaving discovery to
  the session, were both considered and rejected: the first leaves the pull
  requests above unfixed, and the second leaves them unwatched.
- **A stage of its own because the store has to change.** Everything about a
  pull request is keyed one-per-repository today, inline in each table's own
  `CREATE TABLE`, and a stack is several in one repository:
  - `pull_requests` has `UNIQUE (conversation_id, repo_id)`
  - `pull_request_merges` and `pull_request_standings` are
    `PRIMARY KEY (conversation_id, repo_id)`
  - `wrap_up_settled` and `check_fix_attempts` key on
    `(conversation_id, repo_id, …)`
  - `pull_request_checks` keys on the Conversation alone

  Each of those is a rebuild rather than a widening, for the reason
  `crates/store/src/pull_requests.rs` gives about its own constraint: it is
  declared inline, "so it is the table itself that has to be rebuilt, and that
  is not something a `CREATE TABLE IF NOT EXISTS` can reach". So each goes
  through `migrations.rs`, the way `pull_requests_by_repo` and
  `wrap_up_settled_by_repo` already did when a Conversation grew from one pull
  request to one for each repository it reached. Doing it inside stage 06 was
  considered and rejected: five rebuilds plus the chain walk plus a per-stack
  session is two features, and the narrowed wrap-up is worth landing on its own.
- **Keyed by the pull request rather than by the repository.** What replaces
  `(conversation_id, repo_id)` is the pull request the row is about, the
  repository staying on the row as the fact it is. Which is the same move
  `pull_request_merges` records having made once already — "the rollup predates
  a Conversation ending on more than one pull request and is keyed by the
  Conversation alone, and whether a branch merges is a fact about one pull
  request" — and `pull_request_checks`, still keyed by the Conversation, is the
  one that never caught up.
- **The stack is GitHub's chain**, read through `gh` at Start: open pull
  requests of the Repo whose base is another's head, followed until a base is
  not an open head and a head is not another's base. `gh stack`'s own registry
  is per-worktree and may not exist, so it is not what discovery reads; the
  session runs `gh stack init` over the ordered branches where the registry is
  missing, before it syncs.
- **One conflict session per stack**, not per pull request: dispatched when any
  recorded pull request reads *conflicting*, told the ordered list from the
  bottom, told to `gh stack sync` and resolve what it backs out on, run the
  repository's tests, and push the stack — because a fix low in a stack changes
  everything above it. Its goes are counted per stack, two before the stop, the
  stop's Notice naming the pull request left conflicting.
- **The Conversation's own branch is the named pull request's head.** The others
  are branches in the same repository, and the session works them in that one
  Worktree with `gh stack checkout`.
- **`gh stack` is not installed on this machine's `gh`**, and the wrap-up has
  never driven it. The stage verifies the extension is reachable inside the
  Sandbox — or says on the Timeline, by name, that it is not — before anything
  relies on it.

## Proposed tasks (provisional)

1. **Several pull requests in one repository** — the five tables rekeyed off the
   pull request through `migrations.rs`, and every read that assumed one per
   repository found and moved over. AC: a Conversation holding three pull
   requests in one Repo reads all three back with their own checks, merges,
   standings, settles and fix counts; a database written before this opens and
   keeps what it held.
2. **Walking the stack** — the chain read through `gh` both ways at Start, every
   pull request recorded, the Timeline saying what was found. AC: naming the
   middle of a three-deep stack records all three in order; a lone pull request
   records one; a chain that leaves the Repo is not followed.
3. **The settling rule over a stack** — the narrowed wrap-up waits on Mergeable
   and the checks for every recorded pull request. AC: Done comes only once all
   three merge clean and are green; one red check anywhere holds it.
4. **The stack session** — one `addressing` dispatch over the ordered list when
   any recorded pull request conflicts, told to `gh stack sync`, with the goes
   counted per stack. AC: a conflict low in the stack sends one session; it
   pushes every branch; a second conflict after two goes stops with a Notice
   naming the pull request.
5. **The extension check** — the Sandbox is asked for `gh stack` before the
   stack session is sent, and a missing extension is said by name on the
   Timeline rather than found in a session's failure. AC: a Sandbox without it
   stops with a Notice naming the extension; one with it proceeds.

## Re-verify at start

- Stage 06 landed: a Fix Merge Issues Conversation enters Wrapping with Review
  and Comments settled and waits on Mergeable and the checks.
- `pull_requests` still carries `UNIQUE (conversation_id, repo_id)` inline, and
  `migrations.rs` still holds `pull_requests_by_repo` and
  `wrap_up_settled_by_repo` as the worked examples of rebuilding one of these.
- `pull_request_checks` is still keyed by the Conversation alone, so it is a
  two-step move rather than a one-step one.
- `unfinished_pull_requests` and the sweep after Done still read a
  Conversation's pull requests one per repository, so they are among what has to
  move.
- `gh stack sync`, `gh stack init` and `gh stack checkout` still behave as
  `docs/agents/git-workflow.md` describes, and the extension is still a separate
  install.
