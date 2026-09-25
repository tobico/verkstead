# 06. Fix Merge Issues

## Goal

A draft whose Process is **Fix Merge Issues** takes a pull request or a branch
the way Review does, and enters Wrapping with the review and the comments
already settled, so the wrap-up waits on Mergeable and the checks alone: a
conflicting pull request has an `addressing` session sent at it, a failed
check has one, and Done comes when every pull request merges clean and is
green, with no review and no `responding` session ever run. It takes a stack:
at Start the server walks the chain on GitHub from the named pull request in
both directions — a base that is another open pull request's head, a head that
is another's base — records every pull request on the Conversation, and a
conflict anywhere in it dispatches one session told the whole ordered list,
bottom up, which syncs the stack with `gh stack sync` and resolves what it
reports. A lone pull request follows the configured resolution strategy as
today. One role, Implementation.

## Decisions in force

- **[ADR-0020](../../adr/0020-a-conversation-has-a-process.md), *Fix Merge
  Issues***: a wrap-up narrowed to what GitHub refuses a merge for, the
  pre-settled review and comments, the stack walked both ways, one session for
  the stack, `gh stack sync` for a stack whatever the strategy. Walking only
  downward, and leaving discovery to the session, were both considered and
  rejected: the first leaves the pull requests above unfixed, and the second
  leaves them unwatched.
- **Entry is the Resolve-conflicts press's shape widened**: that press already
  enters Wrapping with the review's settle standing and Mergeable unsettled;
  this start settles Review and Comments for every pull request as it enters,
  so the comments watcher has nothing to dispatch and the review watcher
  nothing to start. The checks watcher and the mergeability reading run as
  they always do, per pull request.
- **Every pull request in the stack is recorded on the Conversation**, in the
  same table a companion's pull request is, so the settling rule counts each
  one. The Conversation's own branch is the named pull request's head; the
  others are branches in the same repository, and the session works them in
  that one Worktree with `gh stack checkout`.
- **The stack is GitHub's chain**, read through `gh` at Start: open pull
  requests of the Repo whose base is another's head, followed until a base is
  not an open head and a head is not another's base. `gh stack`'s own
  registry is per-worktree and may not exist, so it is not what discovery
  reads; the session runs `gh stack init` over the ordered branches where the
  registry is missing before it syncs.
- **One conflict session per stack**, not per pull request: dispatched when any
  pull request in the stack reads *conflicting*, told the ordered list from the
  bottom, told to `gh stack sync` and resolve what it backs out on, run the
  repository's tests, and push the stack. Its goes are counted per stack as
  today's are per pull request, two before the stop. A lone pull request is
  today's dispatch, told the configured strategy.
- **Fix Merge Issues on a branch with no pull request** is Review's bare-branch
  path with the same pre-settling: `submitting` opens the pull request, then
  the narrowed wrap-up waits on it.
- **One role, Implementation**, the dropdown-shaped Agent control, readiness on
  a brief, a target and that Pairing. Stage 05's target naming and take-up are
  reused whole.
- **`gh stack` is not installed on this machine's `gh`**, and the wrap-up has
  never driven it. The stage verifies that the extension is reachable inside
  the Sandbox — or says on the Timeline, by name, that it is not — before
  anything relies on it.

## Proposed tasks (provisional)

1. **The narrowed entry** — the Fix start reuses stage 05's take-up and enters
   Wrapping with Review and Comments settled for the recorded pull request.
   AC: no review session starts; a comment landing dispatches nothing; a
   failing check dispatches a fix as today; green and mergeable is Done.
2. **Walking the stack** — the chain read through `gh` both ways at Start,
   every pull request recorded, the Timeline saying what was found. AC: naming
   the middle of a three-deep stack records all three; a lone pull request
   records one.
3. **The stack session** — one `addressing` dispatch over the ordered list when
   any recorded pull request conflicts, told to `gh stack sync`, with the goes
   counted per stack and the stop naming the pull request left conflicting. AC:
   a conflict low in the stack sends one session; it pushes every branch; a
   second conflict after two goes stops with a Notice.
4. **The extension check** — the Sandbox is asked for `gh stack` before the
   stack session is sent, and a missing extension is said by name on the
   Timeline rather than found in a session's failure. AC: a Sandbox without it
   stops with a Notice naming the extension; one with it proceeds.
5. **Fix Merge Issues on the picker** — the row, the dropdown, readiness on one
   role. AC: a Fix draft's Start waits on a brief, a target and one Pairing.

## Re-verify at start

- Stage 05 landed: the target is named and taken up at Start, and a bare
  branch enters Wrapping with `submitting`.
- The wrap-up's settle table is still `wrap_up_settled` keyed by pull request
  with `Review`, `Comments`, `Checks` and `Mergeable`, and the Resolve-conflicts
  path in `resolving.rs` is still the precedent for entering with settles
  standing.
- The conflict dispatch in `checks.rs` still builds its instruction from the
  configured resolution strategy and counts goes per pull request.
- `gh stack sync`, `gh stack init` and `gh stack checkout` still behave as
  `docs/agents/git-workflow.md` describes, and the extension is still a
  separate install.
- A Conversation can still hold several pull requests, one per repository it
  reached, and the settling rule still counts each; holding several in one
  repository is new and the queries that assume one per repository have to be
  found.
