# Fix Merge Issues

A draft whose Process is **Fix Merge Issues** takes a pull request or a branch
the way Review does, and enters Wrapping with the review and the comments
already settled — so the wrap-up waits on **Mergeable** and the checks alone. A
conflicting pull request has an `addressing` session sent at it under the
configured resolution strategy, a failed check has one, and Done comes when the
pull request merges clean and is green, with no review and no `responding`
session ever run.

One pull request here and one role, Implementation: the stack the ADR also
decides on is stage 07, recording several pull requests in one repository being
a schema change across five tables rather than the tail of this one.

Roadmap stage: [06: Fix Merge Issues](docs/roadmaps/processes/06-fix-merge-issues.md)

## Tasks

- [x] 01: Fix Merge Issues starts — [details](01-fix-merge-issues-starts.md)
- [x] 02: The narrowed wrap-up — [details](02-the-narrowed-wrap-up.md)
- [x] 03: A conflict on the way — [details](03-a-conflict-on-the-way.md)
