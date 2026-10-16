# A code icon on the conversation card

Amends [ADR-0024](0024-a-session-suggests-a-preview.md): its line *The header
icon alone: no mark on the sidebar's conversation card* stands for previews and
no longer for everything. The card draws the code icon while something is
active in that Conversation's Code pane.

The human keeps several Conversations going at once and leaves a build running
in one, or an edit half-made in another, and then goes somewhere else. Nothing
on the sidebar said which. The brief asked for the code icon to the left of the
title when a Conversation has something active in Code, and for a press on it
to go straight to the pane.

Decided in the grilling of 2026-10-16 (Question Sets 1182 and 1184).

## What lights it

**A file open in Code with text nobody has saved.** Not git's reading of the
Worktree. The agent's Worktree is dirty for most of a working session, so a git
reading would light nearly every row that already wears the working ring; and
the server's file watcher runs only while a Code pane is attached, so the list
would need a `git status` per row at every read. An unsaved tab is what the
human would lose and what they would come back for. Git's reading alone, and
both under one icon, were rejected.

**A terminal whose foreground is not its shell**: the busy reading a tab's ×
already asks before it ends anything (ADR-0019). Windows cannot read a
foreground and always says busy, so there every open terminal lights the icon,
which was said and accepted. Any live terminal at all was rejected: a shell
sitting at its prompt is nothing anybody would come back for.

**Not a standing preview.** ADR-0024 kept previews to the header's Code icon,
and that stands. The card's icon means *something is active* and nothing else.

## How fresh the terminal half is

**The server reads every live terminal on a turn every two seconds** and
announces a nudge when a Conversation's reading flips. The list is re-read only
on a nudge, and a build finishing in a shell fired none, so without the turn
the icon would have been stale for minutes. Reading it only when the list was
next read for some other reason was rejected for that. **Nothing ticks while no
terminal is live**: the turn starts with the first terminal to open and stops
on the turn that finds none.

**Held until a second turn running before it shows**, in the human's own words:
*Delay showing the icon until it's present for a second turn.* A new terminal
reads busy for its first half-second, while the sandbox wrapper is still in
front of the shell, and the icon flashing on every **New terminal** was not
wanted. So a Conversation's reading goes on only once two turns running read it
busy, and off on the first turn that reads it quiet. The delay is on showing
and not on hiding: a build that has finished is finished. That makes the row's
flag the turn's settled reading, held on the terminals register, rather than a
reading taken when the list is built.

**The `conversation` Nudge**, not `terminals`. The viewer's list re-reads on
it, and so does a hub holding this device's rows for its merged list; neither
listens to `terminals`, so a member's terminal flipping would never have
reached the merged sidebar.

**Defaulted on the wire** (`#[serde(default)]`), so a member running an older
release still merges and says nothing is running.

## How it is drawn

**Quiet**, in the soft ink, the way the working ring is low contrast on
purpose. The accent was rejected: the sidebar keeps it for things the human may
have to do something about, as the header's icon takes it for a preview.

**Before the title, on the title line**, as the brief asked: one icon for both
reasons, `faCode`, the one the Timeline's header opens Code with. The card's
spoken label gains the reason as a clause, and the icon's own label says it for
a pointer: *Code. A terminal is running something.*, *Code. A file has unsaved
changes.*, or both.

**No icon where there is no Worktree.** Terminals cannot exist without one, so
the server's half is right by construction. The unsaved half is the browser's
and can be holding text for a Conversation that has since closed, so it counts
only where the row is neither a Draft nor Closed.

**A member's row draws it too.** The terminal half comes on that device's own
row over the relay, and the press goes to that device's `/code` address.

## The press

**A real link to the Code pane's address**, `pathTo(id, "code", device)`, so
that it is reached by keyboard and opens elsewhere like any link. A plain press
is the workbench's own move: it takes the Timeline out of advancing, as the
header's icon does, and walks a narrow window into the details, which the
address alone would not do where that Conversation is already open.

**The press is the icon's, not the card's**: stopped on its way up, as the
memo's **Retire** and a stage's row are on the Timeline. The pointer going down
on it is stopped too, so a press on the icon is never the start of a drag.

**The card becomes an article carrying the button role**, which `CardButton`
already supports and the Timeline's openable cards already use. A link inside a
`<button>` is invalid HTML.

## Code's storage is keyed by device too

The layout and the unsaved text were keyed by Conversation number alone, so a
member's Conversation 7 and this device's Conversation 7 shared one layout and
one unsaved text, and the card would have lit both. The human chose to fix it
as part of this rather than inherit it.

**The keys follow the URL's shape**: this device's own Conversation keeps the
bare `verkstead.code.<id>` and `verkstead.code-unsaved.<id>`, and a member's
takes `verkstead.code.<device>.<id>` and `verkstead.code-unsaved.<device>.<id>`.
That is the whole migration. The bare keys already stored are this device's own
from now on and lose nothing; a member's Conversation that had written one
starts afresh, which was the ambiguous case anyway. The keeping is keyed by
device and number together, as Code is handed it.
