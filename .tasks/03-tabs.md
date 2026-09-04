# 03. Tabs

## What to build

The pane holds several terminals, one per tab, and every live one comes back
when the pane is reopened.

The tab bar stands in the pane's head to the right of the title **Terminal**:
the tabs in the order their terminals were opened, then a plus button (Font
Awesome's plus) at the end. Built the way the Output pane's Transcript/Screen
switch is — a group of pressed-or-not buttons rather than a tablist — since that
is the house's answer to the same shape. Each tab is labelled *Terminal N* by
the number the server issued it, which is why the numbers are never reused.

On load the pane draws a tab for every terminal the list endpoint from task 01
answers with and shows the first; where none is live it opens one, as it did.
Plus opens another and shows it. Focus goes to the terminal when a tab is
shown, so typing starts at once.

Every tab keeps its socket open and its xterm mounted, hidden when it is not
the one showing, so a hidden shell's output is there when it is switched to.
Only the tab showing measures the pane and sends its size — a hidden xterm
would otherwise fight the visible one for the pseudo-terminal's size, which is
the very oscillation the Screen's own de-dupe guards against — and switching to
a tab measures it then.

No server work: the endpoints task 01 made are enough.

## Acceptance criteria

- [ ] Plus opens a second shell, which becomes the tab showing, without
      disturbing the first.
- [ ] Reopening the pane, or reloading, shows every live terminal as a tab, in
      the order opened, labelled by number.
- [ ] Switching tabs sends a size for the tab now showing and none for the
      hidden ones, and a hidden tab's output is there when it is shown.
- [ ] Web tests over the existing fake socket cover the three rules above.
