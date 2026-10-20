# 03. The Go menu

## What to build

A **Go** menu between View and Window — the Mac's own order is File, Edit,
View, Go, Window, Help — drawn on a Mac only:

- **Back** `⌘[` and **Forward** `⌘]` — the window's own navigation history,
  which on a page that routes in place is every pane and page the human has
  walked through. Greyed when there is nothing behind or ahead, and kept true
  on every navigation the window reports, in-page ones included.
- a rule, then **Conversations** — the workbench with nothing open (`/`), the
  way back from the settings page without the mouse. No key: ⌘0 is the zoom
  reset (Set 1283 Q8b).
- a rule, then **Timeline** `⌘1` · **Code** `⌘2` · **Backlog** `⌘3` ·
  **Roadmap** `⌘4` · **Share** `⌘5` — the workbench's own order, on the keys
  a Mac gives an app's main views (Q8a, Q9). Each is enabled exactly when task
  02's latest push names it, and a press visits the path that push gave for it
  over the channel the other acting items use. On the settings and compose
  pages all five are greyed; on a conversation whose record has no task list,
  Backlog is; with no roadmap landed, Roadmap is.

**Enabling is a value.** Given the latest `Where` and the two history flags,
which items are enabled is decided by a module vitest runs; the edge applies
that answer to the menu Electron holds — by toggling the built items' `enabled`
by id, or by setting the menu again, whichever reads better — and never decides
it. A push before the window exists, or between pages, just updates the value
the next press reads.

**On the keys.** The human chose ⌘[ and ⌘] knowing Monaco binds them to
outdent and indent in the Code pane's editor, and accepted the editor losing
those keys if the menu wins (Q10). It may not: Electron on a Mac hands a
keystroke to the page before the menu, so an editor that takes ⌘] while it has
the focus most likely keeps it, and Back fires everywhere else — Xcode's
behaviour for free. Whether that is so cannot be seen from Linux. Prove it on a
Mac if one is reachable — a throwaway workflow on a macOS runner is how this
repository has proved Mac behaviour before — and otherwise leave a memo saying
it is unproven; the pick stands either way. Nothing in the page binds ⌘ with a
digit, so the five pane keys collide with nothing.

Back and Forward on their own, on the record: `webContents.navigationHistory`
is the API, its deprecated `goBack`/`canGoBack` on the contents being the ones
Electron's typings mark as going.

## Acceptance criteria

- [ ] ⌘1 on a conversation with Code open shows the Timeline with the details
      pane closed; ⌘2 opens Code; ⌘3 and ⌘4 are greyed until the record has a
      backlog and a roadmap, and ⌘4 opens the latest roadmap by name; ⌘5 opens
      Share; on a member's conversation each lands under its device.
- [ ] On the settings page the five pane items are greyed and Conversations
      returns to the workbench; Back then returns to the settings page, and
      Forward from the very start of a run is greyed.
- [ ] A desktop test pins the Go menu's labels, ids and accelerators and its
      place between View and Window, and the pure module's answer for each of
      the cases above; the Windows and Linux templates are still unchanged.
- [ ] A memo records whether the editor keeps ⌘] while focused, proven on a
      Mac or stated as unproven.
- [ ] `pnpm lint`, `pnpm typecheck` and `pnpm test` pass in `desktop/`.
