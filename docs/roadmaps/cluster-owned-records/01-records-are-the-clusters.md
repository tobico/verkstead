# 01. Records are the cluster's — Authors first

## Goal

A replicated-table facility, and Authors as the first record kind on it. An
Author made on any device of the cluster is on every other device once it is
reachable, carrying the same uuid; an edit or a removal made anywhere lands
everywhere, a device that was off catching up when it comes back; an edit that
arrives after a removal stays removed. Nothing is relayed to a home and no row
carries a device mark. Removing an Author that Repos point at names each Repo
with its device, across this device and every reachable member, and offers
*Repoint them all to…*. Demonstrable with two Verksteads: make on A, see on B,
switch A off, edit on B, switch A on, see the edit on A.

## Decisions in force

All from [ADR-0028](../../adr/0028-records-are-the-clusters.md) and the
pointer on the **Author** term in `CONTEXT.md`, which this stage rewrites in
full.

- **A record is a uuid, an `updated_at` and a deletion stamp**, held by every
  device. Newest write wins per record, the device id breaking a tie; a
  removal is final, a removed record never overwritten by a later edit. Per-field
  timestamps and resurrection by a later edit were turned down.
- **A write is stamped past what it was made on top of**: the later of this
  device's now and one past the stamp it holds for the record, so a device
  whose clock runs behind never has its own edit reverted by the next pull.
- **Every device serves its whole table** on a Peer Listener route behind the
  Member Gate — rows and removed records, whoever wrote them, so a record whose
  writer is off still reaches a newcomer through whoever holds it. The others
  pull and merge at start, on the *moved* Nudge, when a member's Nudge stream
  is taken up again and when the membership moves. Removed records are kept.
  Pushing rows on write, owed while unreachable, was turned down.
- **Shaped for the kinds that follow**: Profiles and their bindings (02), the
  Instructions text and MCP declarations (05) go onto the same facility. One
  thing with record kinds on it, rather than four copies of the pattern.
- **Authors leave `config.yaml` for the database**; the token stays in
  `secrets.yaml` under the uuid and is copied to every device as today. The
  uuid an Author already carries is the record's, so nothing a Repo records
  changes. A timestamp-per-entry `config.yaml` was turned down.
- **The token travels from whoever holds one.** With no home, the token route
  answers for any uuid this device holds a token for — its own copy included —
  and a token set or cleared is a write to the record (the `holds_token` fact
  and the timestamp), which is what tells the others to fetch.
- **Nothing merges.** Two Authors with one name and email stay two until the
  human removes one. **Remove on an Author Repos point at** names each Repo
  with its device — this device's and every reachable member's, a member not
  answering skipped and said — and offers *Repoint them all to…* over an
  Author picker; a Repo with a live Conversation refuses it by name, as a
  repoint does today. Checking this device alone, and blocking on an
  unreachable member, were turned down.
- **New release only.** A member too old to answer the route gets the trouble
  line naming both versions (ADR-0027), and its Authors are absent until it
  updates. The `author_mirrors` rows this device holds become records of the
  same uuid stamped older than anything, so the real row wins on the first
  pull.
- **The uniqueness rules are enforced at the press** against the whole table;
  a pair arriving by replication is kept and drawn.

## Proposed tasks (provisional)

1. **The facility.** A store module declaring the record shape generically —
   uuid, `updated_at`, `deleted_at`, the writing device — with the merge rule
   and the stamping rule each one function, the peer route serving a kind's whole table, and the
   puller on the mirroring schedule: start, the kind's *moved* Nudge, a
   member's stream taken up again, membership moved.
   - Two Verksteads: a row written on A is on B after the Nudge; B taken down
     and brought back catches up on the start pull.
   - An edit stamped older than what B holds is dropped; one stamped later
     than the record's deletion stamp is dropped too.
   - An edit on B, whose clock is behind A's, of a record A last wrote is
     stamped past A's write, and A takes it on the next pull.
   - A record made on A reaches C through B after A is unlinked.

2. **Authors on it.** A serve-time rewrite moves `authors:` into the table once
   (the pattern of `record_the_repo_authors`), `config.yaml` no longer read for
   them; `author_mirrors` rows become records stamped oldest; the settings'
   in-memory one list reads the table.
   - A fresh start on a database with mirrors shows one list with no device
     marks and the same uuids.
   - A hand-edited `authors:` key is ignored after the move and said in the log.

3. **Tokens from whoever holds one.** The token route answers any uuid this
   device holds a token for; a set or cleared token bumps the record; the copy
   is fetched on the record's news and taken away with a deletion stamp.
   - A token set on B reaches A's `secrets.yaml` with A's maker off.
   - A removed Author's token copy is gone on every device.

4. **The Git section, the pickers and the edits.** One list, no device marks
   and no *at home on*; Add, edit and Remove are local writes; token
   verification runs where pressed. The relayed `edited`/`removed` and the
   `Pressed::Away` path go.
   - Editing an Author's email on B redraws it on A off the pulled record.

5. **Removal across devices and Repoint them all.** The refusal gathers Repos
   from this device and every reachable member over a peer call, names each
   with its device, skips and says a member not answering, and offers the
   repoint over a picker, refused by name for a Repo with a live Conversation.
   - Removing an Author a member's Repo points at names that Repo and device.
   - *Repoint them all to…* repoints the member's Repo from here and then the
     removal goes through.

## Re-verify at start

- Assumes Authors are still `authors:` in `config.yaml` read by
  `crates/server/src/settings.rs` with `one_list()` joining mirrors from
  `crates/store/src/author_mirrors.rs`, refreshed by
  `crates/server/src/mirroring/authors.rs`.
- Assumes the token still travels on `GET /api/peer/v1/authors/{uuid}/token`
  and is copied into `secrets.yaml` as `author_tokens:`.
- Assumes the Nudge streams in `crates/server/src/relaying/freshness.rs` still
  carry `Nudge::Authors` and send `Everything` on reconnect, and that
  `FailedReads` is still where a member's failed read is recorded.
- Assumes the serve-time rewrites (`record_the_repo_authors` and its siblings)
  are still the pattern for a migration that needs `config.yaml`.
- Assumes ADR-0027's version trouble line and the gate's 403-not-404 are still
  how a too-old member is told apart.
