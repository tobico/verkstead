# 04. Tabs ending

## What to build

A tab goes when its shell ends, the pane never stands empty, and a shell that
could not start does not spawn another.

On the server a shell exiting takes its terminal off the register and closes
every watcher's socket, as a session ending closes its Screen's. The pane reads
a closed socket as the shell having ended.

The rules, all the pane's: a tab whose shell ran for longer than five seconds
goes the moment it ends. When the last tab goes while the pane is open, a new
one opens. A tab whose shell ended within five seconds of the pane asking for
it, or whose open the server refused, stays instead — its last grid read-only
under a line saying the shell ended at once, or the refusal the server answered
with — and nothing opens on its own until plus is pressed, which replaces it.
The five seconds are measured by the pane from when it asked. Nothing happens
while the pane is not open: the server opens nothing of its own accord.

## Acceptance criteria

- [ ] Typing `exit` in the only tab yields a fresh one; in one of several,
      that tab goes and the others stand.
- [ ] A shell that exits within five seconds leaves its tab standing, read-only
      and saying so, and no other opens until plus is pressed.
- [ ] A Sandbox that refuses to start shows the refusal in the tab and spawns
      nothing more.
- [ ] A server test shows an exited shell leaves the register and closes its
      watchers.
- [ ] Web tests over the fake socket cover open-by-default, close-on-end,
      auto-reopen and the guard.
