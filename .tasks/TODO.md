# Wrapping fix

Two settled pieces of work on wrapping up. First, the bug: propose-then-fix
sessions (the wrap-up review and the batch comment sessions) are never ended —
Verkstead waits for an exit that an interactive claude never makes, so the
session idles forever after landing its fixes and the next roadmap stage never
starts. They are to be ended on quiet plus nothing pending, like every other
session kind.

Second, the review's findings grammar goes. A review's Question Set becomes a
plain Set: ways to fix, the spin-off and leave-it are all ordinary Options the
agent words freely, the `review` block leaves the schema, and the server keys
the outcomes on the branch and the session — a fresh `.tasks/` backlog on the
branch is what sends spun-off work back to be built, and a session that dies is
a stop with Resume meaning the review over from the start. The safety net that
dispatched owed fixes from the record goes with the grammar, by decision.

## Tasks

- [x] 01: End a propose-then-fix session on quiet with nothing pending — [details](01-end-on-quiet.md)
- [x] 02: Key the review's outcomes on the branch and the session — [details](02-review-off-the-record.md)
- [ ] 03: Bring the batch sessions under the same rule — [details](03-batches-follow.md)
- [ ] 04: Remove the findings grammar from the schema — [details](04-remove-findings-grammar.md)
- [ ] 05: Rewrite the reviewing and responding skills — [details](05-rewrite-the-skills.md)
