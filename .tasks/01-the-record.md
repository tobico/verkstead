# 01. The Process on the record

## What to build

**A `Process` enum in the store, shaped like `Lifecycle` rather than like
`Direction`.** Five variants — Develop, Investigate, Review, Tinker and Fix
Merge Issues — with the same pair of functions beside them: the word a column
holds, lowercase and spelled out so a database opened by hand says something,
and a read that fails by naming the word it does not know, an unknown one being
a database written by a Verkstead this one does not understand. Its own pair
rather than serde, for the reason `Lifecycle`'s is its own.

All five from this task, though only Develop can start anything. The store reads
and writes every one of them, and what a stage after this adds is a start path
and a row on the picker — never a variant.

Not in the schema crate. `Direction` is there because it rides a Question Set as
a field of `Proposal`, and that crate is the Set and Response grammar the agents
write, compiled to wasm for the browser. A Process is on no Set, and
`Lifecycle` — the per-Conversation fact this is shaped like — is deliberately
not there either.

**A 1:1 side table keyed by the Conversation**, created in the same DDL as
`directions` and for the same reason: `conversations` is `STRICT`, there is no
migration machinery here, and every per-Conversation scalar since has gone
beside the table rather than into it.

```sql
CREATE TABLE IF NOT EXISTS processes (
    conversation_id INTEGER PRIMARY KEY REFERENCES conversations(id),
    process         TEXT NOT NULL
) STRICT
```

One Process per Conversation by the primary key, and a write that replaces
whatever row was there — the upsert `pick_direction` already makes.

**A read that answers whether or not there is a row**, which is the whole of how
Conversations from before this are covered: **Review** where the Conversation is
holding a pull-request adoption, that being the Process its path already was,
and **Develop** otherwise. Read rather than backfilled, so nothing is written
into old rows, and it is the same rule whichever kind of row a Conversation
lacks. It answers a Process rather than an optional one: every Conversation has
one, which is what makes a missing row a reading rather than a gap.

**The loaded Conversation carries it**, beside the Direction — a small read of
its own the way the worktree and the direction are, rather than a `LEFT JOIN`'s
worth of column.

**And the new table joins the delete walk.** The store lists every
Conversation-keyed table there, and a test holds that list against what SQLite
says references a Conversation, so a table left out of it fails the suite rather
than quietly outliving the record it belongs to.

## Acceptance criteria

- [ ] A fresh Conversation reads Develop; one holding a pull-request adoption
      and no row of its own reads Review; and a Process written over either is
      what reads back.
- [ ] All five round-trip through their stored words, and an unknown word fails
      with the word in the message the way an unknown state does.
- [ ] Deleting a Conversation empties the new table, and the test that holds the
      delete walk against the schema passes.
