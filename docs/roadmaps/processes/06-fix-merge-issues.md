# 06. Fix Merge Issues

## Goal

A draft whose Process is **Fix Merge Issues** takes a pull request or a branch
the way Review does, and enters Wrapping with the review and the comments
already settled, so the wrap-up waits on Mergeable and the checks alone: a
conflicting pull request has an `addressing` session sent at it under the
configured resolution strategy, a failed check has one, and Done comes when the
pull request merges clean and is green, with no review and no `responding`
session ever run. One pull request here — the stack is stage 07 — and one role,
Implementation.

## Decisions in force

- **[ADR-0020](../../adr/0020-a-conversation-has-a-process.md), *Fix Merge
  Issues***: a wrap-up narrowed to what GitHub refuses a merge for, with the
  review and the comments pre-settled. The stack the ADR also decides on is
  built in stage 07, for the reason that stage's brief sets out: recording
  several pull requests in one repository is a schema change across five tables,
  and it is a feature of its own rather than the tail of this one.
- **Entry is the Resolve-conflicts press's shape widened**: that press already
  enters Wrapping with the review's settle standing and Mergeable unsettled;
  this start settles Review and Comments as it enters, so the comments watcher
  has nothing to dispatch and the review watcher nothing to start. The checks
  watcher and the mergeability reading run as they always do.
- **Everything about conflicts is today's**, unchanged: one `addressing` session
  per conflicting pull request, told the configured **resolution strategy**, two
  goes counted per pull request before the stop. Nothing about the dispatch
  moves in this stage — what moves is that no review and no comment response
  ever runs beside it.
- **Fix Merge Issues on a branch with no pull request** is Review's bare-branch
  path with the same pre-settling: `submitting` opens the pull request, then
  the narrowed wrap-up waits on it.
- **One role, Implementation**, the dropdown-shaped Agent control, readiness on
  a brief, a target and that Pairing. Stage 05's target naming and take-up are
  reused whole.
- **The Process is offered on the picker from here**, a stack being something it
  grows into rather than something it needs to be offered at all: a Fix Merge
  Issues Conversation over one pull request is the whole of what most of them
  are.

## Proposed tasks (provisional)

1. **The narrowed entry** — the Fix start reuses stage 05's take-up and enters
   Wrapping with Review and Comments settled for the recorded pull request.
   AC: no review session starts; a comment landing dispatches nothing; a
   failing check dispatches a fix as today; green and mergeable is Done.
2. **A conflict on the way** — the existing conflict dispatch runs untouched
   under the narrowed wrap-up. AC: a conflicting pull request gets one
   `addressing` session told the configured strategy; two goes and then the
   stop, the Notice naming the pull request.
3. **Fix Merge Issues on the picker** — the row, the dropdown, readiness on one
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
- The comments watcher and the review watcher both still read their settle
  before dispatching anything, so pre-settling is the whole of what stops them.
