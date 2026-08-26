# 11. Locked Sets

## What to build

Free the word "archive" for conversations by renaming the answered-Set
concept to **locked**, as the human chose: what Question Sets become once
their Response is delivered (or an orphaned Set is put away by hand) is now
"locked", everywhere it faces outward — UI copy, the Set-archiving route and
client function, the generated wire types, the web tests that speak of
archiving, and CONTEXT.md's Archive vocabulary entry, which this task rewrites
to define Locked for Sets (task 12 will add Archive for conversations).

The stored table's name is this task's call: the store has no migration
machinery and existing databases hold the old name, so keeping the table as a
legacy name with a comment is acceptable; renaming it must not break an
existing database.

## Acceptance criteria

- [ ] No user-visible surface, route, client function or exported type still
      says "archived" about a Question Set
- [ ] CONTEXT.md defines Locked and no longer uses Archive for Sets
- [ ] An existing database opens and serves its old put-away Sets as locked;
      Rust and web tests pass
