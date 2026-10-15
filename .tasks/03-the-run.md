# 03. The run

## What to build

A preview knows whether its command is running, and a press on one that has a
terminal does the right one of three things. The server decides, because the
server holds the reading.

**The run.** After typing the command, the server watches the pty's foreground
and **records the first process group that is not the shell's** as the run.
From then on the terminal is read on a turn, as its ports are read while a
member is attached, and what is read is one of three states: the **run** in
front, the **shell** in front, or **something else** in front — a group that is
neither, which is the human having started a command of their own. Where the
platform cannot read a foreground at all, which is Windows, the state is
**unknown**. The reading is the server's own and runs while the terminal lives
and a Code pane is attached, as the files watcher does. A change in it is a
`terminals` nudge, a new kind carrying the conversation and nothing else, and
the Code pane re-reads the terminals view on it.

**The press, decided on the server.** A press on a preview is one request,
naming the preview, and the answer is which terminal to show and what was done
to it:

- no live terminal of this preview's — one is opened and the command typed,
  as task 02 does;
- the run in front — the terminal is refocused and nothing is typed;
- the shell in front — the terminal is refocused and the command is typed
  again, which starts a new run;
- something else in front — a new terminal is opened for the preview, leaving
  the human's command alone;
- unknown — the terminal is refocused and nothing is typed, ever, since a
  command typed into a busy shell would land in whatever is running. Running
  it afresh on Windows is closing the tab first.

Where a preview has more than one live terminal, the newest is the one read.
The viewer chooses the tab the answer names, in whichever group holds it, and
adds a tab for one newly opened.

**What follows the run.** The row's play icon becomes the terminal icon while a
run stands, and goes back to the play icon when the shell or something else is
in front. The tab's title is the preview's while the run stands and is called
as any tab is — the shell's own title, or *Terminal N* — once it ends; it comes
back when the command is typed again. Retiring a preview removes its row and
leaves its terminal as an ordinary tab, called as any tab is. Closing the tab
ends the shell as it always has, and a busy shell still asks first.

## Acceptance criteria

- [ ] Press, Ctrl+C in the terminal, press again: the same tab is refocused and the command runs in it a second time. Press while the run stands: the tab is refocused and nothing is typed.
- [ ] Press after the human has started a command of their own in that terminal: a second terminal opens for the preview and the first is untouched.
- [ ] The row's icon and the tab's title follow the run without a reload, through the `terminals` nudge; retiring a running preview removes the row and leaves the tab, now called by the shell.
- [ ] On a platform where the foreground cannot be read, a press on a preview with a live terminal refocuses it and types nothing.
