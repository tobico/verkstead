# 02. A terminal opened for a preview, and the box

## What to build

The first press. The Code pane draws the standing previews in a box above the
tree, a press opens a terminal and runs the command in it, and the Code icon
on the timeline's header says previews are there.

**Opening a terminal for a preview.** The open-terminal request gains an
optional preview, by title. Opened for one, the terminal is the ordinary
interactive shell in the conversation's own worktree — same account, same
sandbox, same argv — with two things more: the server **holds the preview's
title** on the terminal, and it **types the command** into the pty, followed
by a return, as the human would have typed it. On a Unix it types once the
shell is at the foreground, which is the busy reading answering *not busy* for
the first time — the first moments read as the sandbox wrapper, and a login
shell here can take ten seconds to come up — and never before. On Windows the
pseudoconsole has no foreground to read, so the line is typed at once and the
console holds it until the shell reads it. A title names no standing preview
is a refusal of its own, said in the viewer the way the other named refusals
of that press are.

**On the terminal's view.** Each live terminal says which preview it was
opened for, where it was, and the title it is called by. The viewer calls the
tab by that title ahead of what the shell's own title escape sets, so a
reattach after a reload brings the name back with the tab. Task 03 makes the
title follow the run; here it is the preview's for as long as the terminal
lives.

**The box.** Above the tree, in the sidebar column of Code, a box of its own in
the tree's wash and border, headed **Previews**, listing the standing previews
oldest first. Each row is the title with a play icon at its right, pressable
across its whole width and lit under the pointer the way a row of the tree is.
A press opens a terminal for that preview **in the active group**, exactly
where **New terminal** puts one, and chooses its tab. The box is **not drawn**
where no preview stands; where the pane stacks, under 40rem, the box stacks
above the tree as the tree stacks above the groups, and nothing more is
promised on a phone. Refocusing a terminal already open for the preview is
task 03's; here every press opens one.

**The icon.** The Code icon on the timeline's header takes the accent colour
while any preview stands on the conversation and is the plain icon otherwise.
The open fill it takes while the pane is selected still applies over it, and
it stays disabled where there is no worktree. The header icon alone: no mark
on the sidebar's conversation card.

## Acceptance criteria

- [ ] A press on a preview's row opens a terminal in the active group whose tab is called by the preview's title, and the command runs in it after the shell has come up, with the shell still there when it ends.
- [ ] The tab's title survives a reload, and the terminals view says which preview each live terminal was opened for.
- [ ] A conversation with no previews draws no box and a plain Code icon; one with any draws the box and the icon in the accent, and the box stacks above the tree under 40rem.
