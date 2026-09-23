# 02. The tab bar

## What to build

Code's tab bar is restyled after VS Code's, and two of the Terminal pane's
rules invert (ADR-0019, *Tabs and groups*).

**A kind icon at one end of every tab and a × at the other.** Pressing the ×
on a terminal tab ends its shell, exactly as the context menu's Close did. The
context menu goes: the × is what VS Code has, and a long press is no longer
needed for a menu, which is what frees it for dragging in stage 02 of the
roadmap. Closing a *dirty* file also confirms, but there are no files yet —
that rule arrives with saving.

**The pane opens empty.** It no longer opens a shell of its own accord: a
Conversation's live shells come back as tabs, and where there are none the pane
draws a hint and a **New terminal** button. This is the rule the Terminal pane
inverted — it never stood empty because a shell was the whole of what it held —
and a pane that will hold files has something to show without one. The guard
against a shell that cannot start stays as it is: a shell that ended within
moments of being asked for, or an open the server refused, leaves its tab
standing and saying why, and pressing **New terminal** replaces it.

The empty state is what the pane shows before there is a tree beside it, so it
is drawn where the tabs' content goes rather than over the whole pane.

## Acceptance criteria

- [ ] The pane opens with the Conversation's live shells as tabs and opens
      none of its own accord; a Conversation with no shells gets the hint and
      the **New terminal** button.
- [ ] Every tab draws a kind icon and a ×, and the × on a terminal tab ends
      its shell; the tab's context menu is gone.
- [ ] A shell that could not start, or that ended at once, still leaves its
      tab saying why, and **New terminal** replaces it.
