# 02. Every Repo records its Author

## What to build

The store records which Author each Repo is, by uuid, and the Repo views carry
it.

**A side table beside `repos`**, keyed by `repo_id`, holding the Author's uuid
as text. It sits beside `repos` rather than being a column on it, because
`repos` is STRICT and `unregistered_repos` already takes this shape. It is the
store's own fact. The store knows nothing of `config.yaml`, so whether a uuid
names an Author that still exists is the server's question, not the store's.

**The migration fills it for existing Repos.** The uuid is minted by task 01's
settings migration, which the store's open-time migrations cannot see. So the
fill is a function the serve calls once the settings files are migrated and
before any route is answered: every registered Repo with no row is recorded as
the migrated Author's. `rank_the_conversations` in
`crates/store/src/migrations.rs` is the precedent: filled from outside the open,
called by the serve. An installation with no Author records nothing.

**Registering with exactly one Author configured records it silently**, on Open
repo and on Create repo alike. With none configured, nothing is recorded. With
more than one, nothing is recorded yet: task 05 adds the pick. **Reviving an
unregistered Repo keeps the row it had**: unregistering takes nothing out of
this table, and the association stays as history.

**The Repo views carry the uuid**, the server's Repo view and the render crate's
types the web reads. They also carry one derived fact: *no Author recorded while
more than one is configured*. Stage 02's refusals will act on it; nothing acts
on it here.

## Acceptance criteria

- [ ] A database with registered Repos, opened by a start that migrates the old
      single keys, records every one of them as the migrated Author's.
- [ ] A fresh registration with one Author configured is recorded as that
      Author's, with nothing asked. With none configured, nothing is recorded.
- [ ] Unregistering a Repo and registering the same path again keeps the Author
      it had.
- [ ] A Repo with no recorded Author and two Authors configured is stated as
      such by its view.
- [ ] The store's own tests cover the table, the fill and the revive.
