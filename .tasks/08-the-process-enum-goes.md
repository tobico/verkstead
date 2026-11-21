# 08. The store's `Process` enum goes

## What to build

A Process is a reference to a manifest everywhere: the store's five-variant
`Process`, the render crate's mirror and its generated TypeScript type go, as
does every `match` on them in the server. What reads "which Process" reads the
plugin uuid and Process id (task 02), and asks the manifest whatever it wanted
to know — takes a Target, which roles, which state is first.

The five bare words stored before this stage stay readable as aliases of the
built-in plugins; nothing in the `processes` table is rewritten.

## Acceptance criteria

- [ ] No `Process` enum remains in `crates/store` or `crates/render`, nor its
      generated type in the viewer
- [ ] A database from before this stage reads every Conversation's Process as
      it did, including those with no `processes` row (Develop)
- [ ] The whole Rust suite and the web suite pass
