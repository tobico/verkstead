# 02. Groups and drags

## Goal

The one group stage 01 built becomes any number: a tab's menu, an icon at the
end of the tab bar, or a tab dragged to an edge splits the group beside or
below itself, any depth, with dividers that drag. Tabs drag between groups and
back, files drag in from the tree, and a group whose last tab leaves is gone.
A file opened in two groups is one buffer seen twice. A reload comes back to
the same groups, the same tabs and the same unsaved text, and the browser-safe
shortcuts move between tabs and open splits and terminals.

## Decisions in force

- **[ADR-0019](../../adr/0019-the-code-pane.md), *Tabs and groups*** is the
  whole of the rulebook: splits any depth, the three ways to make one, the
  empty group vanishing — which is the whole of unsplitting, there being no
  command for it — one buffer under two views, no preview tabs, and the layout
  kept in the device's storage per Conversation.
- **The layout is a tree**, split nodes with a direction and a share per child,
  groups at the leaves, and it is the one thing the pane draws from: what is
  showing, where the dividers are and which group is active are all read off
  it. The shares are percentages of the parent for the reason the frame's pane
  widths are — a divider dragged on a laptop should mean the same on a wider
  screen — and the frame's own divider is the pattern for how one is dragged,
  nudged by keyboard and clamped.
- **Drags are pointer events**, the way the sidebar's cards are dragged, with
  the pointer captured and the drop decided by where it is let go: over a tab
  bar it is an insertion point, over a group's content it is one of five zones
  — the centre moves the tab in, each edge splits there. The sidebar's card
  drag is the pattern; the file attachments' drop is not, that being the
  browser's DataTransfer for files from outside the page.
- **One Monaco model per file path across the pane**, held with the buffers
  above the pane rather than in any editor, so two views share text, dirty
  state, the version and the save. A model goes when its last view closes.
- **What the device remembers** is the layout tree, each group's tabs and its
  active one, and the dirty text of every dirty buffer, keyed by Conversation
  in `localStorage` beside the pane widths and the wrap setting. On load,
  terminals in the layout are reconciled against the server's live list and
  ended ones dropped; a dirty buffer restored is dirty against the version the
  file is read at now, which is what makes the reload bar right on a file that
  moved while the page was away. Storage is a convenience the whole way down,
  as `device.ts` says: a browser that refuses it costs the layout and nothing
  else.
- **Shortcuts are those a browser lets through**: Ctrl+PageUp and Ctrl+PageDown
  between a group's tabs, Ctrl+\ to split right, Ctrl+` for a terminal in the
  active group. Ctrl+W and Ctrl+Tab are the browser's and are not taken.

## Proposed tasks (provisional)

1. **The layout tree** — the pane draws from a split tree; split right and
   split down from a tab's menu and from the tab-bar icon; a group whose last
   tab goes disappears; the active group is the one last pressed into. AC: two
   splits nest; closing the last tab in a nested group collapses it and the
   neighbour takes the room; a new file opens into the active group.
2. **Dividers** — draggable and keyboard-nudged, clamped to a floor per group.
   AC: a drag changes the shares; the floor holds; a resize of the window keeps
   the shares.
3. **Dragging tabs** — reorder within a bar, move between bars, and drop on a
   content zone to move in or split there, with an insertion marker and a zone
   highlight. AC: a tab dropped on the right edge splits right with that tab
   alone in the new group; a drag that returns to where it started changes
   nothing; the terminal a moved tab shows keeps its socket.
4. **Dragging from the tree** — a file row picked up the same way opens in
   the group or zone it is dropped on. AC: a drop on a content centre opens the
   file there; a drop on an edge splits; a drop elsewhere does nothing.
5. **One buffer, two views** — the same file in two groups shares text and
   dirty state and saves once. AC: typing in one shows in the other; the dot is
   on both tabs; one save clears both.
6. **The device remembers** — layout, tabs and dirty text restored on reload,
   terminals reconciled, dirty text held against the current version. AC: a
   reload comes back to the splits with the unsaved text; a terminal ended
   while away is not in the layout; a dirty file that moved while away shows
   the bar.
7. **Shortcuts** — the four above, with the terminal's own keys untouched
   while a terminal tab has focus. AC: Ctrl+PageDown cycles; Ctrl+\ splits;
   Ctrl+` opens a shell; Ctrl+C in a terminal still interrupts.

## Re-verify at start

- Stage 01 landed with Code's state held above the details pane, which is
  where the buffers and the layout tree hang off.
- The sidebar's card drag is still pointer-event based with capture, and its
  long-press handling — the pattern here.
- `localStorage` on the device is still the home of per-device settings via
  `device.ts`, and the key namespace is still `verkstead.*`.
- The busy flag on terminals and the × confirm still live where stage 01 put
  them, so a moved terminal tab keeps them.
