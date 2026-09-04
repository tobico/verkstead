# 01. One terminal, end to end

## What to build

The whole path once, and thin: a press on the Timeline's head lands the human
in a shell running in the Conversation's Sandbox, and what they type goes there.

A **Terminal** icon button (Font Awesome's terminal icon) stands beside Share in
the Timeline head's controls, drawn as Share is: the same `IconButton`, open
while its pane is what the details are showing, absent in a share. Where the
Conversation has no Worktree — a Draft, a Closed one — it is drawn disabled and
its label says there is no worktree yet.

Pressing it opens a details pane of its own at `/conversations/:id/terminal`,
a word opening beside `share` and `backlog`. The pane has the usual sticky head,
titled **Terminal**. Under it the terminal fills the pane: the 60rem reading
measure every details pane pads its content to comes off, the way the composer
takes it off, and the pane takes the Screen's fixed-height rule so the terminal
is sized to the pane rather than scrolling it. The viewer is the Screen's xterm
and socket handling, reused rather than copied — always live, typing on, no
fetched branch — and it measures itself and sends its size exactly as the
Screen does. One screen however many devices watch it, sized by whoever resized
last.

On the server, a register of the Conversation's terminals beside the sessions'
own — several per Conversation, so a register of its own rather than a bend in
the one-session-per-Conversation map — each holding a `Terminal`, the sandboxed
child and a `Live` screen, keyed by a number the server issues in order and
never reuses for that Conversation. Three endpoints under
`/api/ui/conversations/{id}/terminals`: open, which starts one and answers its
number; list, which answers the numbers of the live ones; and
`{n}/attach`, the websocket, carrying the Screen's own message shapes down
(a painted grid, then printed bytes) and up (a resize, checked on the way in
as the Screen's is, and what was typed). An attach to a number that is not
live is refused as a Screen's is. Nothing is written to the store and nothing
appears on the Timeline.

The shell is `/bin/sh`, interactive, until task 02 chooses better. It runs in
the Sandbox `Sandbox::for_conversation` builds for the implementation pairing's
Profile — the grilling pairing's where the implementation role has none — with
everything a session gets: the Worktree as the working directory, the git
directory, the handoff directory, the build cache, the Sandbox Configuration
binds, `GH_TOKEN`, the git author and `VERKSTEAD_SERVER` scoped to this
Conversation. Wrapped in `nix develop --command` where the worktree's flake
provides a dev shell, as a session's command is. A platform with no
pseudo-terminal refuses to open one, the way it refuses to run a session.

On load the pane asks for the list, attaches to the first live terminal, and
opens one where none is live — so a reload comes back to the same shell. The
tab bar is task 03; here there is one terminal and no bar.

## Acceptance criteria

- [ ] Pressing Terminal on a Conversation with a Worktree opens the pane at
      `/terminal` holding a shell whose working directory is the Worktree, and
      what is typed into it is answered.
- [ ] Reloading the pane attaches to the same shell, still running, showing
      what it last showed.
- [ ] The button is disabled with a reason on a Draft and on a Closed
      Conversation, and is not drawn in a share.
- [ ] The terminal fills the pane's width and height: no reading measure and
      no page scroll.
- [ ] A server integration test opens a terminal in a real Sandbox, types
      `pwd`, and reads the Worktree back off the grid the way the screen tests
      read theirs.
- [ ] Nothing is written to the store and no Event appears on the Timeline.
