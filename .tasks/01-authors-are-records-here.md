# 01. Authors are records, here

## What to build

The **Replicated record** facility, on one device first, with Authors as its
first kind — and Authors out of `config.yaml` for good. Nothing crosses the
network in this task; the existing Author mirrors keep working beside it
exactly as today, so a cluster behaves as it did.

**The facility** is a generic store module (`replicated` in the store crate;
the server side, when it needs one, is `replicating` — `records` is already
taken by OpenCode's store reader). It declares the record shape once for every
kind, so Profiles, their bindings, the Instructions and MCP declarations can
follow onto it in later stages without a copy of the pattern:

```text
uuid        TEXT PRIMARY KEY   -- the record's, the same on every device
updated_at  stamp              -- of the last write
deleted_at  stamp or NULL      -- the deletion stamp; a removal is final
written_by  device id          -- breaks a tie between equal stamps
...kind's own fields
```

Two rules, each one function, unit-tested on their own:

- **Merge**: per record, the newest `updated_at` wins, the writing device's id
  breaking a tie; a record carrying a deletion stamp is never overwritten by a
  later edit, and a deletion stamp arriving beats any edit. Removed records are
  kept, never pruned.
- **Stamping**: a local write is stamped with the later of this device's now
  and one past the stamp this device holds for that record, so a device whose
  clock runs behind never has its own edit reverted by the next pull.

**Authors on it.** An Author record is its uuid plus display name, git name,
email and `holds_token` (whether this device's `secrets.yaml` holds a token
for it — the fact, never the token). At serve time, after the existing
single-`git_author` rewrite (`moving_the_single_author` /
`record_the_repo_authors` — the pattern to copy), `authors:` is moved out of
`config.yaml` into the table once, with the uuids it already carries, so
nothing a Repo records changes; the key is then dropped from the file. After
the move `config.yaml` is no longer read for Authors: a hand-edited `authors:`
key is ignored and the log says so. The settings' in-memory list of Authors —
what `Config::authors`, `author_of`, `author_missing` and the one list resolve
against at launch — is fed from the table, the way the mirrors are laid down
today, and refreshed on every write.

**Every write goes to the table**: the settings save (the Git section sends
the one Author it drew there, merged by `keeping_what_the_page_never_drew`),
the onboarding wizard's first Author, the per-Author edit and the Remove — a
removal is a deletion stamp, and the Author's token leaves `secrets.yaml` as
today. Tokens stay in `secrets.yaml` under the uuid.

Add **Replicated record** to `CONTEXT.md` as a term: what the facility is, the
three columns, the two rules, and that it holds every kind ADR-0028 names.

## Acceptance criteria

- [ ] An installation with `authors:` in `config.yaml` starts with the same Authors under the same uuids, now out of the database, and the key gone from the file; a Repo's recorded Author still resolves.
- [ ] A hand-edited `authors:` key added after the move is ignored, and the log says it was.
- [ ] Adding an Author in the Git section, one from the wizard, an edit and a Remove are each a write to the table, stamped by the stamping rule; a removed Author is a row with a deletion stamp, absent from every list and picker.
- [ ] Unit tests of the merge rule: newest wins; equal stamps break by device id; an edit stamped after a deletion stamp is dropped; an edit older than what is held is dropped.
- [ ] A unit test of the stamping rule: with a clock behind the held stamp, a write is stamped one past it.
- [ ] Author mirrors still show and resolve as before; nothing on screen changes on a device in a cluster.
- [ ] **Replicated record** is a term in `CONTEXT.md`.
