# 08. Detail panes

## What to build

Migrate the workbench detail panes — Commit, Document, PullRequest, Output and
Screen — each to its colocated module, moving their blocks out of `main.css`.
The panes' shared chrome is already `PaneHead` (task 04); this task is each
pane's own body.

Particulars, settled in grilling:

- Commit and Diff share the diff rendering: those classes are global after
  tasks 01–02, referenced as `:global(...)` where the pane refines them.
- Output's transcript styles markdown inside turns — the
  `.transcript .prose .markdown` refinements become module rules with
  `:global(.markdown)`.
- Screen wraps xterm: xterm's stylesheet stays a plain import and its grid
  classes are never renamed; anything of ours that reaches into xterm's DOM
  does so via `:global`.
- Data-driven flags (`live`, `failed` on tool-result turns, wrap toggles) go
  through the module object.

Tests (`workbench`, `diff`, `opening`, `relaying` and any other naming these
classes) import the modules.

## Acceptance criteria

- [ ] Commit, Document, PullRequest, Output and Screen each have their module;
      their blocks are gone from `main.css`.
- [ ] Commits and PRs render as before; the transcript's turns and markdown are
      unchanged; the live terminal still fits and refits its grid.
- [ ] `pnpm typecheck`, `pnpm lint` and `pnpm test` pass.
