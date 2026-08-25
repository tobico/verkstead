# 04. Pane chrome component

## What to build

The workbench pane furniture — `.pane-head` (7 components), `.pane-back` (6)
and `.close-event` (4), styled once in `main.css` — becomes a `PaneHead`
component in `web/src/workbench/` with its own colocated module. It owns the
header bar's layout, the way back out of a pane, and the close control, taking
the varying parts (title content, the back target, extra controls) as props or
children — read the seven current call sites first and shape the props from
what they actually vary.

All seven pane components adopt it; the pane-chrome rules move into
`PaneHead.module.css` (camelCase); descendant refinements in unmigrated
components survive via the `class`-prop pattern from task 03 or a `:global`
reference, whichever reads better at each site. Tests asserting on the old
class names import the module.

The `.pane`/`.pane-chrome` layout rules that belong to the Workbench's own grid
stay in `main.css` for now — they move with the Workbench in task 09. Only the
chrome the seven panes repeat moves here.

## Acceptance criteria

- [ ] One component renders the pane chrome everywhere it appears; the moved
      rules are gone from `main.css`.
- [ ] Pane headers, back links and close controls look and behave as before,
      including the pinned-header behaviour while a pane scrolls.
- [ ] `pnpm typecheck`, `pnpm lint` and `pnpm test` pass.
