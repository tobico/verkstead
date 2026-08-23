# 01. Query wrapper and lint wall

## What to build

Make ADR-0005's reconcile-or-static rule unskippable. Introduce a
project-owned query helper that every viewer query goes through, whose
signature forces each caller to make the freshness decision explicitly: name
the key `reconcile` merges re-reads by, or declare the payload `static`
because it cannot change. No silent default — a query that says neither does
not compile. Conditional staticness (a finished session's record) must remain
expressible.

Migrate every existing query through the wrapper: the four that already chose
(conversation, transcript, and the two set queries) keep their choices; the
seven bare ones (conversations, repos, profiles, abandoned roadmaps,
pull-request, screen, capture) each get an explicit decision. List payloads
merge by the id flat on each element — abandoned roadmaps carry `repo_id`
rather than `id`, so that query names it. Judge pull-request, screen and
capture on their own payloads: merge what re-reads while live, freeze what
cannot change.

Add a lint wall so the rule holds for queries not yet written: importing the
query hook directly from the data-layer package fails lint everywhere except
inside the wrapper's own module.

This slice is demonstrable on its own: with the wrapper in place, a Nudge
refetch no longer rebuilds list DOM — conversation rows keep their nodes (the
working spinner keeps spinning), and dropdown options survive identity-intact.

## Acceptance criteria

- [ ] Every viewer query goes through the wrapper; each names a reconcile key
      or declares static, and the type signature makes omitting both an error
- [ ] Lint fails on a direct query-hook import outside the wrapper module,
      and the existing lint setup runs it
- [ ] With a running session nudging, conversation-list rows and dropdown
      option elements are not recreated on refetch (verify by hand or with a
      quick check; the pinned regression tests are task 02's)
- [ ] Existing vitest and Rust suites pass unchanged
