# 09. Workbench shell

## What to build

Migrate the workbench's outer shell and left column: Workbench (the three-pane
layout, the pane divider and its drag handle, and whatever `.pane`/
`.pane-chrome` grid rules task 04 left behind), Conversations (the list, its
rows, marks and menus), Setup and Adoption. Each gets its colocated module;
`panes.ts` keeps working against the renamed classes.

Conventions as settled: camelCase identical in CSS and TS; selection, open and
stage flags through the module object; Conversations imports Mark's module for
the mark it draws rather than duplicating it; the `⋯` menus keep the Menu
`class`-prop pattern, with the caller-side refinements now living in these
callers' modules.

Tests (`workbench`, `choosing`, `archiving`, `nudging`, `pwa` and any other
naming these classes) import the modules.

## Acceptance criteria

- [ ] Workbench, Conversations, Setup and Adoption each have their module;
      the workbench-layout and conversations blocks are gone from `main.css`.
- [ ] The phone-first single pane, the wide three-pane layout, and the divider
      drag all behave as before; conversation rows, marks and menus are
      visually unchanged.
- [ ] `pnpm typecheck`, `pnpm lint` and `pnpm test` pass.
