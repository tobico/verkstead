# 06. Titles

## What to build

A tab is called what its shell calls itself.

xterm reports the title a shell sets through the terminal title escape, and the
tab's label follows it; where the shell has set none, or sets an empty one, the
label is the *Terminal N* of task 03. A fresh attach starts from the number
again — a repaint carries the grid and not the title — until the shell sets one
again, which most prompts do at every prompt.

## Acceptance criteria

- [ ] A shell that sets its title relabels its tab, and clearing the title
      returns the number.
- [ ] A tab whose shell has set no title reads *Terminal N*.
- [ ] A reload reads the number until the shell next sets a title.
- [ ] A web test feeds a title escape through the fake socket and sees the tab
      relabelled.
