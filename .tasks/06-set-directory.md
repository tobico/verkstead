# 06. The set/ directory

## What to build

Migrate the Set page: every component under `web/src/set/` gets its colocated
module, and the shared question vocabulary gets one home. Decisions settled in
grilling:

- **`Sheet.module.css` owns the question vocabulary** — `.question`,
  `.options`, `.option`, `.star`, `.answer-table`, `.direction-card` and the
  rest of the block that both Sheet and its child Answering render.
  `Answering.tsx` imports Sheet's module for those classes; Answering-only
  classes live in `Answering.module.css`. One definition, no duplication.
- The other files — SetPage, Contents, Diff, Postscript, Standing, Unreadable,
  AskText, table — each get their own module for their own classes.
- Classes applied to server-rendered HTML (`.markdown`, the diff family,
  `section-heading` anchors) are global after tasks 01–02: reference them from
  modules as `:global(...)` where a component context refines them.
- Functions that return class names as data — `face()` in `outline.ts`, the
  liveness badge, the answered-row marks — return the camelCase module names,
  looked up through the module object (`styles[value]`); every variant the data
  can produce must exist in the module, since a missing one silently drops
  styling.

Tests (`answering`, `choosing`, `contents`, `diff`, `set`, `sizing`,
`surviving`, `resuming`, `drafting`, `outline` and any other that names these
classes) import the modules they assert on.

## Acceptance criteria

- [ ] Every `set/` component has its module; the Set page's rules are gone from
      `main.css`.
- [ ] The question vocabulary is defined once, in Sheet's module, and the
      answering view and the read view still render identically styled rows.
- [ ] `pnpm typecheck`, `pnpm lint` and `pnpm test` pass; the Set page —
      preface, questions, tables, diff, postscript, standing badge — is
      visually unchanged.
