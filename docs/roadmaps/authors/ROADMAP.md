# Authors roadmap

One person, several names on their work. An **Author** is who a Repo commits and
pushes as — a git name, an email and an optional GitHub token — and an
installation may hold several of them, each Repo naming which is its. A session
is handed its Repo's Author and nothing of any other, so an agent working a
pseudonymous repository has neither the name nor the token of the one beside
it. The decisions and their why are in
[ADR-0025](../../adr/0025-a-repo-has-an-author.md); the term is **Author** in
[CONTEXT.md](../../../CONTEXT.md), beside **Repo**, **Agent Profile** and
**Companion Repo**, which each stage keeps current as its piece lands.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

Strictly ordered. 01 turns the one author into a list and makes a second
Author work end to end on one device, which is the whole of what the two use
cases need where there is one machine. 02 stands on 01's store and pickers: it
is the screens and the refusals that make the pick visible and keep two Authors
from mixing, and the last of Verkstead's own git calls moving onto the Repo's
Author. 03 stands on both: the cluster mirrors what 01 defined and relays the
edits 02 built, and its preflight compares the picks 02 makes changeable.

## Stages

- [x] 01: The author list — [brief](01-author-list.md) — no dependencies
- [ ] 02: The pick on screen and the rules — [brief](02-pick-and-rules.md) — after 01 *(in progress: `roadmaps/authors/02-pick-and-rules`)*
- [ ] 03: One list across the cluster — [brief](03-one-list-across-the-cluster.md) — after 02
