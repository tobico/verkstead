# A session suggests a preview

Amends [ADR-0019](0019-the-code-pane.md): a terminal can be opened for a
command the agent suggested, and the Code pane lists those suggestions above
its tree.

ADR-0019 built Code "for trying the agent's work and finishing it by hand".
Trying it starts in a terminal with a command — `cargo run -- serve`, `npm run
dev`, a script under `scripts/` — and which command is a thing the agent knows
and the human has to go and find: in the transcript, in the diff, in a README
the agent may or may not have updated. The Code pane opens empty and offers
**New terminal**, and the first thing typed into it is a guess. The human asked
for the agent to say how, up front: a **preview**, listed in the pane and
started with a press.

Decided in the grilling of 2026-10-15. The backlog that builds it is
`.tasks/` on the `preview-scripts` branch.

## A title and a command, suggested by the agent

**A preview is a title and a one-line command**, left by a building session
with `verkstead preview "Build and test"` and the command on stdin — the shape
`verkstead memo` has, with the one thing a memo does not need, a name, as the
argument. It is suggested **the moment there is anything that could be
tried**, whether or not the agent thinks a manual test is called for: the
point is that it is already there when the human decides one is. Enforced by
nothing — a task that touched only documents has nothing to preview, and a
done signal refused for want of one would be the server deciding what is
worth trying.

**Keyed on its title.** The same title suggested again replaces the command in
place. The agent suggests early and the command changes as the work does, and a
list that gained a near-duplicate on every task would be one nobody read. A
row per suggestion, retired by id as a memo is, was rejected for that reason.

**Taken back by the agent alone**, with `verkstead preview --retire <title>`.
A **Remove** on the row was considered and left out: the list is the agent's
reading of how to try its work, and a human who finds a row stale has a
terminal to say so in and a session to steer. Nothing here stands in the way of
adding it.

**Building sessions only**, by the memo's rule and off the same register the
done signal is checked against: a backlog task, the finish, a stage's planning,
an inline run, a steered instruction. A grilling has nothing to try and an
investigate session commits nothing.

**One line, written for the terminal's own shell.** The command is typed into
an interactive shell, so it is one line of at most 4 KiB, refused with a
newline in it, and it is written for the shell it will be typed into — the
machine's login shell on a Unix, Windows PowerShell on Windows — which the
session's prompt names, the agent being unable to see it from inside the
sandbox. Wrapping the command in `sh -c` so the agent could write POSIX
everywhere was rejected: what the human sees typed into their fish or zsh
would be a quoted wrapper rather than the command. A script file the server
writes and the terminal runs was rejected for the same reason one step on —
what is typed would be a path.

**Stands until retired.** Through a steer into a new grilling and through a
follow-up, as a memo does: how to try the branch is true of the branch rather
than of the round.

## Stored with the conversation, and not an event

A side table on the conversation — title, command, who left it, when — carried
on a transfer and deleted with the conversation. **Not a timeline event**: no
card when one is suggested or replaced, and so no place on a share. ADR-0023
made a memo an event because nothing on a conversation happens off the record,
and a preview is the agent's own act in the same way; but a card for every
replacement of *Build and test* as the work moves would be the timeline
recording the agent's housekeeping, and what a preview is *for* is a press in
the Code pane rather than a reading of the record. Memory only, as a terminal
is, was rejected: a suggestion lost with a server restart is one the agent has
to be asked for again.

Served on the conversation view beside the timeline, so the header icon can
read it without the Code pane open, and announced by the `conversation` nudge.

## Typed into a terminal, as the human would have

**A press opens an ordinary terminal and types the command into it**, followed
by a return, once the shell is at the foreground — which is the busy reading
answering *not busy* for the first time, the first moments after spawn reading
as the sandbox wrapper. The human sees the command, its output stays, the up
arrow runs it again, Ctrl+C stops it, and the shell is still there afterwards.
A terminal that *is* the command — a new argv per preview — was rejected
twice over: run alone, the tab ends with the command and a failed build takes
its output with it; run and then dropped into a shell, what the human sees is a
wrapper's doing rather than their own.

**The run is the foreground group.** After typing, the server records the first
process group in the pty's foreground that is not the shell's, and reads it on
a turn for as long as the terminal lives and a Code pane is attached, the way
ports are read while a member is attached. Three states come of that reading —
the run in front, the shell in front, something else in front — and a change
is a `terminals` nudge. Comparing the foreground process's *name* with the
command's first word was rejected: `npm run dev` has `node` in front.

**And the press is decided on the server**, which holds that reading:

- no live terminal — open one and type;
- the run in front — refocus the tab;
- the shell in front — refocus and type again;
- something else in front — open a new terminal, and leave the human's
  command alone.

Windows has no foreground to read, so a live terminal there is refocused and
never typed into again: a line typed into a shell that might be busy lands in
whatever is running. The command is typed at once on open there, the
pseudoconsole holding it until the shell reads.

**The tab is called by the preview's title while the run stands**, held by the
server so a reattach brings it back, and the shell's own title escape does not
replace it; when the run ends the tab is called as any tab is, and the title
comes back when the command is typed again. Retiring a preview leaves its
terminal, as an ordinary tab.

## The box, and the icon

**A box of its own above the tree**, in the tree's wash and border, headed
*Previews*, each row the title with a play icon at its right that becomes the
terminal icon while its run stands; the whole row is the press, lit as a row of
the tree is. **Not drawn where none stand**: every conversation starts with
none, and a box saying so on each would be noise where a box appearing says
the thing. Rows inside the tree's own box, and a strip across the top of the
pane, were both considered; the box is its own thing and reads as one.

**A press opens the terminal in the active group**, where **New terminal**
puts one. Under 40rem the box stacks above the tree as the tree stacks above
the groups, and nothing more is promised on a phone.

**The Code icon on the timeline's header takes the accent while any preview
stands.** The first header icon to carry state in its colour; the checks ring
and the back link are the precedents for the colour. Amber until the pane is
next opened, on the pattern of **Unseen**, was rejected — the colour means
*there is something to try*, which stays true — and amber only while a run
stands was rejected as saying something the tab already says. The header icon
alone: no mark on the sidebar's conversation card.

## What the agent is told

A section of the Guide, a short section in each building skill pointing at it,
and a heading in every building session's prompt listing the standing previews
with their commands — shown **when the list is empty too**, over a hint to add
one, which is where this differs from the memos' heading: the empty heading is
the nudge the brief asked for. Written into the prompt alone, with no Guide
section, was rejected as the memo's teaching was placed: the Guide is where
the verb's contract is quoted, and a skill is read before a prompt is.

## The word

**Preview**, defined in `CONTEXT.md`. *Script* says what the command is in and
not what it is for; *task* is a backlog's; *launcher* and *run configuration*
are an IDE's.
