# 09. One menu component, one shadow

## What to build

The workbench has three dropdown menus built three ways: the
per-conversation ellipsis menu (native details/summary), the new-conversation
menu (hand-rolled, invisible backdrop, Escape handling, focus management)
and the set-standing menu (hand-rolled, no shadow). Unify them on one shared
menu component — trigger, positioning, backdrop, Escape-to-close and focus
return in one place — and give all menus one shared, stronger drop shadow
than today's (two already carry a soft one; it was settled they should all
stand out better against background content).

Keep each menu's contents and behaviour otherwise as it is. The component is
also what the next task's settings menu will be built from.

## Acceptance criteria

- [ ] All three menus render through the shared component and visibly share
      chrome and shadow
- [ ] Escape closes each menu and returns focus to its trigger; a click
      outside closes it
- [ ] Each menu's actions behave exactly as before
