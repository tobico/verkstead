# 01. Directory listing endpoint

## What to build

A UI-facing endpoint that lists **one directory per request** — the browse
dropdown fetches lazily, level by level, so there is never a whole-tree walk.
Model it on the existing UI routes (the repo branches listing is the nearest
shape).

It answers in two **scopes**, which are the two kinds of field it will serve:

- **watched** — the scope for values the server refuses outside the Watched
  Paths. Asked for no path, it answers the watched roots themselves; asked for
  a path, it lists it only where admission (the same decision the server
  already makes, on the resolved path) puts it inside a watched root, and
  refuses it otherwise.
- **anywhere** — the scope for values with no such restriction, rooted at `/`.
  Any directory the server can read lists; this wider disclosure was decided
  deliberately in the grilling.

Each entry says what it is, because the field drawing it behaves differently
per kind: a **directory**, a **file**, or a **repository** — a directory
holding `.git`, which the Repos' form will draw marked and never descend into.
Entries are sorted directories-first, then by name; dotfiles are always
included (the client decides per field whether to show them).

A directory that cannot be read — permissions, gone between requests — answers
as an ordinary refusal the page can draw, never a 500.

The wire types follow the project's rule: defined in `verkstead-render`, added
to the TypeScript-generation root list so `cargo test` rewrites the viewer's
types, and a fixture written from the server's own tests the way the existing
UI-content fixtures are.

## Acceptance criteria

- [ ] Listing a directory inside a watched root returns its entries with kinds,
      directories first; the watched scope with no path returns the watched
      roots.
- [ ] A path outside every watched root is refused under the watched scope and
      listed under the anywhere scope; a directory holding `.git` comes back
      marked as a repository.
- [ ] An unreadable or vanished directory answers a drawable refusal, and the
      generated TS types and a server-written fixture land with the change.
