# 02. Rename the renderer's own classes

## What to build

The repo's convention after this migration is camelCase for every class name we
write ourselves. Apply it to the classes `crates/render` emits by hand —
`diff-file`, `diff-hunk`, `diff-lines`, `diff-path` and the rest of the diff
family — end to end: the Rust that writes them, the server tests that assert on
them, `web/src/styles/diff.css`, and every web component and web test that
names them.

The documented exception, decided during grilling: library-generated names stay
as the library makes them. Syntect's `tok-*` output (the `tok-` prefix is ours,
the suffixes are generated from scope names), mermaid's DOM and xterm's DOM are
not renamed and never will be — no post-processing of generated HTML. Leave a
short comment where the exception is visible (the `tok-` prefix constant and
`diff.css`) saying so.

## Acceptance criteria

- [ ] No kebab-case class name written by our own Rust or referenced in
      `diff.css` remains, except the `tok-*` family.
- [ ] `cargo test` passes (if the local `guide` test fails only under
      `VERKSTEAD_SERVER`, run with `env -u VERKSTEAD_SERVER` — a known
      machine-local quirk, not this change).
- [ ] `pnpm typecheck`, `pnpm lint` and `pnpm test` pass; diffs render
      visually unchanged.
