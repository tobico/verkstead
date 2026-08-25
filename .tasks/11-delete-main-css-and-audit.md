# 11. Delete main.css and audit

## What to build

Finish the migration. After tasks 01–10 whatever still sits in `main.css` is
either misplaced or dead: place each remaining rule where the conventions say
it belongs (a component's module, one of the global sheets, or a variable), or
delete it as dead, then delete `main.css` itself and its import.

Then audit the whole result:

- Every class defined in every `*.module.css` is referenced by its component
  (or a deliberate importer, like Answering using Sheet's vocabulary and
  Conversations using Mark's) — unreferenced classes are deleted.
- Every `styles.x` reference in TS resolves to a class that exists — a missing
  one silently drops styling, so sweep the data-driven lookup sites
  (`styles[value]`) against the value sets their types allow.
- No kebab-case class name written by us survives anywhere — the only
  exceptions are the documented library names (`tok-*`, mermaid, xterm).
- The global sheets hold only what can never be hashed; nothing component-owned
  hides in them.

## Acceptance criteria

- [ ] `main.css` no longer exists; the app builds and serves from the modules
      and the three global sheets alone.
- [ ] The audit sweeps above pass, with anything they caught fixed in this
      task.
- [ ] `pnpm typecheck`, `pnpm lint` and `pnpm test` pass, `pnpm build`
      succeeds, and the human has given the running UI its final visual pass —
      that pass is the migration's last gate.
