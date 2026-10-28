# 02. Replicated across the cluster

## What to build

The facility's network half, with Authors replicating on it, and the Author
mirrors retired.

**Every device serves its whole table of a kind** — live rows and removed
records, whoever wrote them — on a Peer Listener route behind the Member Gate:
`GET /api/peer/v1/records/{kind}`, Authors being `authors`. Never a token.
This replaces the way Authors cross today, which is a member reading the
viewer's own `/api/ui/authors` over the Peer Listener; members on this release
stop reading that.

**Every device pulls and merges** each member's table with task 01's merge
rule, on the mirroring schedule: at start, on the kind's *moved* Nudge
(`Nudge::Authors`), when a member's Nudge stream is taken up again
(`Nudge::Everything`), and when the membership moves. A pull is idempotent, so
a missed Nudge costs nothing. A local write announces the kind's Nudge. A
merge that changed anything refreshes the settings' in-memory list and redraws
the pages. A record whose writer is off or unlinked still reaches a newcomer
through whoever holds it.

**A member too old to answer the route** — it passes the Member Gate and
answers `404`, where the gate's refusal is `403` — is a failed read recorded
in `FailedReads`, whose trouble line names both versions under ADR-0027; its
Authors are absent until it updates.

**The mirrors go.** Each `author_mirrors` row this device holds becomes a
record of the same uuid stamped older than anything, so the real row wins on
the first pull; then the `author_mirrors` table and the Author-mirroring
refresher are retired. The settings' list is the table alone. A Repo pointed
at a pulled Author whose token has not been copied is refused with today's
*token awaited* words — token copies arrive in task 03, and that gap is
accepted for this task's length.

The proofs are two- and three-Verkstead suites, the way the cluster suites
already stand two servers up.

## Acceptance criteria

- [ ] The demonstration: an Author made on A is on B after the Nudge; A switched off, the Author edited on B, A switched on — A shows the edit after its start pull.
- [ ] B taken down while A makes an Author, then brought back, has it after its start pull.
- [ ] An edit on B, whose clock is behind A's, of a record A last wrote is stamped past A's write, and A takes it on the next pull.
- [ ] An Author removed on A and edited on B while apart stays removed on both once they meet.
- [ ] An Author made on A reaches C through B after A is unlinked.
- [ ] A fresh start on a database holding mirrors shows one list with the same uuids, and the real rows replace them on the first pull; `author_mirrors` is gone.
- [ ] A member answering `404` on the route leaves a trouble line naming both versions.
