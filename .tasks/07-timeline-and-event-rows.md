# 07. Timeline and event rows

## What to build

Migrate the Timeline — the single biggest component (about 2,000 lines, 86
distinct classes) — together with the small components that live inside its
rows: Mark, Pause and Asked. Each gets its colocated module; the timeline-event
blocks of `main.css` (event cards, the clamp, the question-set rows, the moved/
resumed lines, task-list and stage-list rendering) move with them.

Conventions as settled: camelCase names identical in CSS and TS; data-driven
classes — `classList={{ [resumed().by]: true }}`, `moved.state.toLowerCase()`,
selection and waiting flags — go through the module object, with every variant
present in the module file; markdown injected into events is styled globally
and refined via `:global(.markdown)` where the timeline context needs it; Mark
is imported where Conversations currently repeats its classes only if that
falls out naturally — Conversations itself migrates in task 09.

Tests (`workbench`, `resuming`, `surviving`, `archiving`, `nudging` and any
other asserting on timeline classes) import the modules.

## Acceptance criteria

- [ ] Timeline, Mark, Pause and Asked each have their module; their blocks are
      gone from `main.css`.
- [ ] Event rows, selection states, the clamp's cut behaviour, pause/resume
      lines and the marks render exactly as before, in light and dark.
- [ ] `pnpm typecheck`, `pnpm lint` and `pnpm test` pass.
