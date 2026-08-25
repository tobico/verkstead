# 05. Root and small components

## What to build

Give each of the small, self-contained components its colocated module and move
its rules out of `main.css`: Menu, Modal, Switch, App, UpdateNotice
(`update/`), and Notifications (`push/`). These are the components with a
handful of classes each, so this task proves the whole per-component pattern at
low stakes before the big directories.

Conventions (set in task 03, restated because each task runs in a fresh
session): camelCase class names identical in CSS and TS; a component that takes
a `class` prop keeps taking one, and the parent styles it from the parent's own
module; rules matched by nothing die here; comments move and may be rewritten.
Menu and Modal are the components *receiving* passed classes — their own
structural classes (`menu`, `modal`, backdrop, rows) move to their modules,
while the per-caller refinements stay where the callers are until those callers
migrate.

Tests (`menus`, `modals`, `notifications`, `update`, and any other that names
these classes) import the module and build selectors from it.

## Acceptance criteria

- [ ] Each listed component has a colocated `*.module.css` and no longer
      depends on `main.css` for its own classes.
- [ ] Menus open and position as before; the modal draws, closes and traps
      focus as before; the switch and update notice are visually unchanged.
- [ ] `pnpm typecheck`, `pnpm lint` and `pnpm test` pass.
