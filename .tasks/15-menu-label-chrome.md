# 15. Menu and label chrome

## What to build

Three small chrome fixes, settled together:

- **Conversation menu subtitles.** The sentence describing each action moves
  inside its menu item as a subtitle, instead of sitting after the item. The
  transient notes go entirely — the stopping status line, the
  this-conversation-has-been-closed line, and the inline failure lines; a
  failed action logs a `console.error` for debugging and draws nothing, since
  actions are not expected to fail in regular use. Both menus built from the
  same rows (the pane-head menu and the sidebar right-click) change together.
- **Main menu.** Settings first, then the archived toggle, relabelled **Show
  archived** and prevented from ever wrapping to a second line.
- **The Direction label.** The chooser on a closing Set is labelled **Final**
  instead of **End** — one constant, drawn in both the live chooser and the
  settled record.

## Acceptance criteria

- [ ] Every conversation-menu action carries its description inside the item;
      no notes render outside items, and a failed action logs to the console
      and draws nothing
- [ ] The main menu reads Settings first, then a one-line **Show archived**
      toggle
- [ ] The Direction card's hanging label reads **Final** in the chooser and on
      the record alike
