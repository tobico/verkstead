# 05. A rewritten commit comes off the Timeline

## What to build

A Repo whose resolution strategy is **rebase** has every commit rewritten under
a new sha when its conflict is resolved. The sweep only ever adds, so the work
lands a second time beside the originals and the Timeline reads as though it
were done twice. An amended commit does the same on any Repo.

So a sweep also subtracts: a recorded commit the branch no longer carries is
forgotten, and the rewritten one records fresh in its place.

Which ones those are is one git read for the whole set, not one per commit:

    git rev-list --ignore-missing --no-walk <every recorded sha> --not <branch>

What comes back is exactly the recorded shas the branch no longer carries.
`--ignore-missing` is what stops a sha that has since been garbage collected
failing the whole read; such a sha is simply not reported, so its Event stays,
which is the safe way round.

**Ancestry, not the listing.** What decides this is whether git still holds the
commit on the branch — never whether it survived task 01's exclusion. A branch
whose pull request has been merged is wholly reachable from its base branch, so
a Conversation swept by the listing would take its entire history off its own
Timeline the first time a Follow-up session ran. It also means the base-branch
commits already recorded on old Timelines stay where they are, which is what was
decided about them.

Forgetting an Event means the Event and everything hanging off it — the commit
row and its Commit Summary — in one transaction, and the page told, so a
Timeline open on it re-reads and a details pane on a forgotten Event recovers
rather than sitting on nothing.

## Acceptance criteria

- [ ] After a rebase, each commit is on the Timeline exactly once, under the sha
      the branch now carries.
- [ ] An amended commit replaces its original rather than joining it.
- [ ] A commit the branch still carries is never forgotten, including one the
      base branch has swallowed — a Conversation whose branch was merged into
      its base keeps its whole Timeline when a later session sweeps it.
- [ ] A recorded sha git no longer holds at all leaves its Event alone.
- [ ] The page is told when something is forgotten.
- [ ] The sweep's module doc no longer says a rewritten branch leaves commits
      no sweep has seen.
