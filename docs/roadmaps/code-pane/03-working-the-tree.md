# 03. Working the tree

## Goal

A row in the tree has a menu — a right-click — with new file, new folder,
rename and delete, each done by the server and reflected in the tree and in
any tab the path is open in. Ctrl+P opens a quick-open field that matches
file names across every root and opens the pick into the active group. The
pane's menu carries word wrap, font size and minimap, remembered per device
and applied to every editor.

## Decisions in force

- **[ADR-0019](../../adr/0019-the-code-pane.md), *The tree* and *Monaco,
  whole*** — the four row actions, quick open, no find-in-files, and the three
  settings exposed.
- **Each action is an endpoint of the files API**, refusing in the body the
  way the reads and writes of stage 01 do: a name that already exists, a root
  (which cannot be renamed or deleted), a read-only root, a path escaping one.
  Rename is a move within a root and never across two — two roots are two
  repositories.
- **The menu is the terminal tab's context menu again** — the one `ContextMenu`
  the app has, with which row it is about held by the tree rather than by a
  menu per row. **The mouse's alone**, settled when this stage was planned:
  stage 02 landed first and made a long press on a file row the way it is
  dragged into a group, so a touch screen gets no row menu rather than a second
  gesture invented for one — which is how the sidebar's cards answer the same
  problem, and Code is desktop-first besides.
- **A name is typed in place**, as an inline field on a new row or over the
  renamed one, rather than in a modal: what is being named is a row in the
  tree, and the tree is where it is seen.
- **An open tab follows its file.** A renamed file's tab is retitled and its
  buffer re-keyed; a deleted file's tab stays, read-only, saying the file is
  gone, with its text still there to be copied out — a tab that vanished under
  somebody with unsaved text would take the text with it.
- **Quick open is a palette over a file list the server gives per root**, git's
  own list of tracked and untracked-not-ignored files, read when the palette
  opens and matched on the page; the picker components the app already has
  are the shape, and the match is subsequence with a preference for path
  segments, which is what VS Code's feels like at this size.
- **Settings live in `device.ts`** beside the Diff's wrap setting, each read
  by every editor mounted, and drawn as rows on the pane's own menu in the
  header — the place a pane keeps what is about the pane.

## Proposed tasks (provisional)

1. **New file and new folder** — endpoints, menu rows on a folder and a root,
   the inline name field, and the new file opening into the active group. AC:
   a new file appears in the tree and in a tab; a name that exists is refused
   with a sentence; a read-only root offers neither row.
2. **Rename** — endpoint, menu row, inline field over the row; open tabs
   follow. AC: a renamed open file's tab is retitled and saves to the new
   path; renaming across roots is impossible to ask for; a folder rename
   carries its children's tabs.
3. **Delete** — endpoint with the confirm the app uses for what cannot be
   taken back; a deleted open file's tab stays read-only saying so. AC: a
   folder deletes with its contents after one confirm; the open tab keeps its
   text; the tree row is gone.
4. **Quick open** — the per-root file list endpoint and the Ctrl+P palette.
   AC: typing part of a path lists matches across roots with the root named;
   Enter opens into the active group; Escape leaves nothing behind.
5. **Settings** — wrap, font size and minimap on the pane's menu, per device,
   on every editor. AC: a change applies to every open editor at once; a
   reload keeps it; a second device is untouched.

## Re-verify at start

- The files API of stage 01 still refuses in the body, and its root listing
  still says which roots are writable.
- `ContextMenu` still takes both hands, and the tree's rows are not yet
  dragged by long press — if stage 02 landed first, a long press on a row is
  a drag, and the menu on a touch screen needs a row of its own to open from.
- Whether stage 02 has landed: a deleted or renamed file's tab may be in any
  group, and the buffers may be shared across two views.
