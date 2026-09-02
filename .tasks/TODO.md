# Timeline own commits

A Conversation's Timeline gains a commit for everything on `<base commit>..<branch>`,
which means a resolution session that merges the base branch in drags every commit
the base has gained since the work was cut onto the Timeline with it. None of that
is the Conversation's work, and it buries what is.

This makes the rule explicit: a commit is the Conversation's when the base branch
does not already hold it. The base branch's *name* gets recorded beside the commit
it resolved to, the sweep excludes what that branch carries, the merge commit stays
as a row that shows the hunks the agent actually resolved, and a rebase's rewritten
commits stop landing twice.

## Tasks

- [x] 01: The sweep leaves out what the base branch already holds — [details](01-exclude-the-base-branch.md)
- [ ] 02: A merge commit says what was resolved — [details](02-merge-shows-its-resolution.md)
- [ ] 03: The renderer reads combined diffs — [details](03-render-combined-diffs.md)
- [ ] 04: The merge row says it is a merge — [details](04-label-the-merge-row.md)
- [ ] 05: A rewritten commit comes off the Timeline — [details](05-forget-rewritten-commits.md)
