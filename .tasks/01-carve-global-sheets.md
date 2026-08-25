# 01. Carve the global sheets

## What to build

Split the styles that can never be component-scoped out of `web/src/main.css`
into plain global sheets under `web/src/styles/`, imported once from the entry
point in place of nothing — `main.css` stays imported too and shrinks by what
moved:

- `base.css` — the `:root` design tokens, the dark-mode override, element
  defaults, the prose measure, and any other rule that styles elements rather
  than classes.
- `markdown.css` — the `.markdown` typography for server-rendered markdown
  injected via `innerHTML`, and the mermaid diagram styles (mermaid draws its
  own DOM; its class names are its own).
- `diff.css` — the diff rendering (`.diff-*` family) and the syntax-highlight
  colors for syntect's `tok-*` classes, both emitted as HTML by `crates/render`
  and injected via `innerHTML`.

A pure move: no renames in this task (task 02 renames), selectors and behaviour
unchanged. Comments may be trimmed or rewritten where moving makes them stale.
Rules that turn out to be matched by nothing are deleted, not moved.

## Acceptance criteria

- [ ] The three sheets exist, are imported from the entry point, and the moved
      blocks are gone from `main.css`.
- [ ] `pnpm typecheck`, `pnpm lint` and `pnpm test` pass in `web/`.
- [ ] The running UI is visually unchanged — same rules, new homes.
