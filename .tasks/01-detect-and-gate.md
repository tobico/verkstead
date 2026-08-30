# 01. Detect conflicts and gate Done on them

## What to build

Teach a wrap-up whether each of its pull requests can merge cleanly, and make
being conflict-free one of the things Done waits for.

The checks watcher already asks `gh pr view --json statusCheckRollup,headRefOid`
every poll, per pull request, the Conversation's own and each read-write
companion's alike. Add `mergeable` to that same call — the grilling settled
that detection rides this poll rather than getting a watcher of its own, so it
costs no extra `gh` call. GitHub answers one of three ways and each means its
own thing:

- **MERGEABLE** settles the fact.
- **CONFLICTING** unsettles it.
- **UNKNOWN** (GitHub still computing) changes nothing at all — the same
  honesty the watcher already applies to a `gh` that cannot answer: *Verkstead
  does not know* is a third thing beside yes and no.

Record the reading per pull request in a table of its own, keyed by
conversation and repo. Do not ride the existing check-rollup table: it keys per
Conversation alone (it predates companions), and this fact is a pull request's.
The record survives a restart for the reason the rollup does — later tasks draw
an indicator and a button off it long after any watcher has stopped.

Add a new settle-fact variant beside `Checks`, `Review` and `Comments` — one
per pull request, carrying the repo id the way `Checks` does — and fold it into
the rule that ends a wrap-up. Every watched PR must be mergeable before the
Conversation reaches Done, which also closes the race where a conflict appears
just as the last suite goes green.

The narrowing that writes **Waiting on checks** must count the new fact: a
wrap-up whose PR is conflicted is not merely waiting on GitHub, so it does not
narrow.

This task is detection and gating only — nothing is dispatched at a conflict
yet. A conflicted wrap-up simply waits, exactly as a red suite with no goes
would. Task 02 adds the resolution.

## Acceptance criteria

- [ ] A wrap-up whose pull request GitHub reports CONFLICTING does not reach
      Done, and settles once GitHub reports it MERGEABLE again.
- [ ] UNKNOWN and an unanswerable `gh` neither settle nor unsettle the fact.
- [ ] The recorded mergeable state is per pull request, survives a restart,
      and companions' pull requests are covered like the Conversation's own.
- [ ] A wrap-up with a conflicted PR is not marked Waiting on checks.
