# 04. Memory unioned in and fanned out

## Goal

The memory sync no longer has a home. Before a launch under a Profile whose
memory switch is on, this Repo's and this Worktree's entries are pulled from
every reachable device that holds a store for the Profile — a bound account or
a copy — the newer file winning; after the session, what it wrote is pushed to
every reachable device. Nothing is deleted anywhere. OpenCode's one database is
taken newest whole. A device not answering is a line on the Timeline, never a
gate. Demonstrable with three Verksteads: a session on A, one on B, and a
session on C that reads both transcripts.

## Decisions in force

From [ADR-0028](../../adr/0028-records-are-the-clusters.md); this stage
rewrites the **Memory sync** term in full.

- **Union in, fan out after.** A memory-device field on the record, synced to
  and from as the home was, and no memory across devices at all, were turned
  down.
- **Newer file wins**, so each file travels with its modification time and
  the receiving end keeps the newer; a file still hashing to what arrived is
  not sent back, the rule the sync already follows.
- **Nothing is deleted anywhere**: a file a session took away is not one it
  asked another device to lose.
- **The unit per harness stays** as ADR-0020's *Memory sync* set it: Claude's
  two entries, Codex's and Grok Build's picked-out sessions, OpenCode's one
  database with its two siblings — which cannot be unioned and is taken newest
  whole, newest by the database file's time.
- **A label apiece and each machine names its own path**: the Repo's entry
  through the Repo match, the Worktree's off the Data Directory stem, as today.
- **Best effort rather than a gate**: a device not answering is a Notice
  naming it, in and out.
- **The bound on a store** stays at the sync's sixty-four megabytes, per
  device answered.

## Proposed tasks (provisional)

1. **Who holds a store.** The memory route answers for any Profile this device
   holds a store for — its bound account or its copy — and the asking side
   lists every reachable device rather than a home.
   - B and C both answer for a Profile bound on A and copied on them.

2. **Union in.** Pull from every reachable device, each file with its time,
   newer wins into the store the root is built from; a device not answering
   is a Notice.
   - A session on C reads a transcript written on A and a memory file written
     on B.

3. **Fan out.** Push what changed to every reachable device; nothing deleted;
   a device not answering is a Notice.
   - A memory file written on C is on A and B afterwards; a file C deleted is
     still on A.

4. **OpenCode newest whole.** The database with its siblings taken from the
   device whose copy is newest; pushed back to every reachable device after.
   - Two OpenCode sessions on two devices end with the newer database
     everywhere.

## Re-verify at start

- Assumes 02 landed: bindings and copies named by uuid; 03 may or may not have.
- Assumes `crates/server/src/mirroring/memory.rs` is still the sync with the
  per-harness parts in `Root::synced`, the SHA-256 rule and `MOST_A_STORE_IS`,
  and `crates/server/src/peer/memory.rs` still answers at
  `profiles/{profile}/memory` and `/memory/left`.
- Assumes the Repo match in `crates/server/src/matching.rs` is still origin
  then name.
- Assumes the harness-side records reader still stops rather than writes where
  OpenCode's schema is unknown (the one-column exception for a carried
  Conversation aside).
