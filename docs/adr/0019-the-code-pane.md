# The Code pane

Amends [ADR-0013](0013-conversation-terminals.md): the Terminal pane grows into
a small editor, and the server reads and writes a Conversation's Worktree on
the workbench's behalf.

ADR-0013 opened a shell in the Conversation's Sandbox "for the moment the
agent's work is done and somebody wants to try it, make a small change or work
with git". Trying it takes a shell. Making the small change takes an editor,
and the one a shell offers is whatever is installed inside the Sandbox, driven
through xterm in a browser — which is nobody's idea of one. So the Terminal
pane becomes **Code**: a file tree over every Worktree the Conversation has,
files edited in Monaco, and terminals as tabs beside them, in groups that
split. A desktop tool for finishing a Conversation's work by hand; nothing is
promised on a phone, though the pane still draws there.

Decided in the grilling of 2026-09-23. The roadmap that builds it is
`docs/roadmaps/code-pane/`.

## One pane, replacing the Terminal

Code is a details pane at `/code`, opened by the `code` icon where the terminal
icon stood beside Share, and `/terminal` redirects to it. There is no Terminal
pane any more: a shell is a tab of Code, and a Conversation's live shells come
back as tabs whenever the pane is opened, exactly as they came back to the
Terminal pane. Two panes, one holding shells and one holding shells and files,
would be the same thing drawn twice.

A **maximise** toggle in the pane's header hides the sidebar and the Timeline
on a window wide enough to draw them, because an editor wants room and the
details pane is what two other panes leave. Off when Code first opens, and the
last state is remembered per device beside the pane widths. A whole-window mode
outside the three-pane frame was considered and rejected: the frame is where
every other pane's dividers, widths and back-links live, and a second frame for
one pane would be all of that written again.

## The server reads and writes the Worktree, outside the Sandbox

The Sandbox bounds what a *session* may reach. The workbench is the human, who
already reaches the whole Worktree through a terminal in it, and the server
already reads the Worktree on the workbench's behalf — the Diff on a Set, the
backlog, the roadmaps. So the files API reads and writes as the server, with
no Sandbox in front of it, and what bounds it is the same thing that bounds a
shell: the roots. A root is the Conversation's own Worktree or a companion's;
a read-only companion is read-only here as well; a path that escapes a root,
or names anything under `.git`, is refused. Routing writes through a shell in
the Sandbox was rejected — a boundary built for an agent, put around the
human's own hand, at the cost of a process per save.

**What keeps a session out of it is the Workbench Key**
([ADR-0015](0015-open-boundary-and-workbench-key.md)). A session's network is
the host's own, so nothing about the socket tells a session's request from the
browser's, and "the workbench is the human" is a claim the key is the whole of:
it is kept in the Data Directory, which no Sandbox binds, and everything under
`/api/ui/` is refused without it. That is where the files API goes — beside the
terminals' routes — and it is the widest thing the key has stood in front of
yet, a read and a write of any file in any of the Conversation's Worktrees. A
files API reachable without the key would hand a session the reach the Sandbox
was built to take away, so the gate is named here rather than left to be
inferred from the prefix.

On Windows a session runs as an account of Verkstead's own, granted entries on
the Worktree, and a file the server writes there is written as the human. A
file the session cannot then read would be the one thing that breaks this, and
whether the granted entries are inherited by a new file is verified at the
first stage rather than assumed here.

## Versioned reads, and a stale write is refused

The agent may be writing the same file. A read hands back the content with a
**version** — a hash of what was read — and a write says which version it is
over. A write over a file that has moved since is refused, and the editor is
what chooses: a clean editor reloads on its own when the disk moves, and a
dirty one keeps the human's text and shows a bar offering *Reload* or *Keep
mine*, after which a save is a write over the version it now knows. Last
writer wins was rejected: an agent's edit silently overwritten by a human who
never saw it, or the other way about, is exactly the collision this exists to
surface.

Saving is explicit — Ctrl+S, a dot on the dirty tab, a confirm on closing a
dirty tab — which is VS Code's default and the one the reload bar depends on:
an autosave has no dirty state to hold the human's text in while they decide.

## Following the disk

The disk moves under the pane: the agent writes, a build runs, a terminal tab
checks out a branch. A **watcher** on the server announces it as a `files`
Nudge, which carries a Conversation and nothing else (ADR-0009), and the page
re-reads the folders it has expanded, the git status it draws its marks from,
and the files it has open. The watcher is on demand — it runs while a Code
pane is attached to the Conversation and not otherwise — and it walks the tree
ignore-aware, because a recursive watch over a Rust `target/` exhausts a
machine's inotify watches on its own. Polling from the page, and a refresh
button in place of a watcher, were both considered: the first is the poll
ADR-0009 took out of the viewer, and the second is the human doing the
watcher's job.

## The tree

One root per Worktree — the Conversation's own first, then each companion —
with git-ignored paths and `.git` hidden, and git status marks on files and
their folders, so what the agent changed is visible before a diff is. A row's
menu holds new file, new folder, rename and delete. Quick open (Ctrl+P) matches
file names across the roots. Find-in-files is deferred: a terminal tab and
`grep` cover it until it is missed.

## Tabs and groups

The tab rules of ADR-0013 are amended.

- **× on every tab**, files and terminals alike, with a kind icon at the other
  end. Closing a dirty file confirms. Closing a terminal ends its shell, and
  confirms first where the shell is **busy** — its foreground process is
  something other than the shell itself, which the server reads off the pty it
  holds. What it reads is that process's *name* rather than its identity:
  `tcgetpgrp` on the pty answers with a pid, and busy is that pid running
  something other than the shell the terminal was started with. Comparing the
  pid against the shell's own was considered and cannot be done — the child
  the server holds is the sandbox wrapper, the shell is a grandchild inside
  its own pid namespace and behind the worktree's dev shell besides, and the
  wrapper is the pty's session leader as well, so the shell's own number never
  reaches this side. A platform where it cannot tell confirms every time.
  ADR-0013 kept Close on a context menu because a × beside a small label is
  easy to hit; that stands as the reason a busy shell asks, and the × is what
  VS Code's tab bar has and what this one is styled after. A long press is no
  longer needed for the menu, which is what frees it for dragging on a touch
  screen.
- **The pane no longer opens a shell on load.** It opens empty, with a hint and
  a New terminal button, and live shells come back as tabs. The Terminal pane
  never stood empty because a shell was the whole of what it held; a pane that
  holds files has something to show without one. The guard against a shell that
  cannot start stays: its tab stays saying why.
- **Groups split**, any depth, side by side or stacked, with draggable dividers
  between them. A split is made from a tab's menu, from an icon at the end of a
  tab bar, or by dragging a tab to an edge of a group; a group whose last tab
  goes disappears and its neighbour takes the room. Tabs drag between groups,
  and files drag in from the tree.
- **And unsplitting is that last tab leaving.** The Brief asked to split and to
  unsplit, and this is the whole of the second: a group goes by having its tabs
  closed or dragged into another, and the split it stood in collapses when the
  last of them does. An unsplit command of its own was considered and left out
  — it is a third way to reach what two gestures already reach, and unlike them
  it would have to be found on a menu first.
- **The same file opened twice is one buffer under two views**, dirty together
  and saved together. No preview tabs: every open is a real tab.
- **What is open is the device's.** Groups, tabs, the active one and the dirty
  text are kept in the browser's storage per Conversation per device, so a
  reload comes back to the layout with the unsaved text still in it, and a pane
  swapped away for an Event and back finds its tabs where they were. A terminal
  that has ended since is dropped from the layout on load. Kept on the server
  was rejected for the reason the pane widths are not: a device's layout is a
  device's, and a phone that opened Code has nothing to say about a laptop's
  splits.

## Monaco, whole

The whole of the editor package — every built-in language coloured, and
completions and diagnostics for TypeScript, JavaScript, JSON, CSS and HTML from
its bundled workers — as chunks the page loads when Code first opens and never
before, served under the hashed `/assets/` path the viewer already keeps for a
year. The Brief asked for Monaco and for VS Code's feel, and the cost is a
download the rest of the workbench never makes. VS Code's defaults, following
the workbench's light and dark themes; word wrap, font size and minimap are the
three settings exposed, on the pane's menu, per device.

**And a second cost, which is the binary's.** The viewer is embedded in the one
binary ([ADR-0004](0004-single-binary-distribution.md)), which already counts
the embedded viewer among the named costs of that decision — so Monaco is not
only a download the page makes once, it is weight in every release everybody
downloads and every update everybody takes, and the bundled TypeScript worker
carries the TypeScript compiler and is the largest file of it by some way. It
is kept whole regardless: the Brief asked for VS Code's feel, and an editor
trimmed of the languages somebody happens to open is a worse pane in exactly
the moment it is wanted. What the trim would be, should a release ever have to
go on a diet, is the TypeScript worker rather than the languages — colouring is
most of the pane's value and almost none of its weight.

**What it came to, measured** on the release build that first carried it
(monaco-editor 0.56.0, `cargo build --release -p verkstead-cli` on Linux
x86-64): the binary went from **48.7 MB to 62.9 MB**, so Monaco is **14.2 MB**
of every release and every update. Of that, the editor chunk is 4.0 MB, its
stylesheet and icon font 0.3 MB, and the five workers 9.5 MB — of which the
TypeScript one alone is **7.0 MB**, just over half the editor's total weight
and the one thing a diet would take. The workbench's own chunk grew by 1.4 kB,
which is the whole of what a page that never opens Code pays: the editor is
fetched when Code first opens and never before.

An image opens as a preview in its tab; any other binary, and any file over a
few megabytes, opens as a line saying why rather than in the editor.

## Forwards

Trying the agent's work is very often starting a dev server in a terminal tab
and opening it in a browser — and where the Conversation is a member's
([ADR-0020](0020-cluster-mode.md)), the server is listening on the other
machine's `localhost` and the browser is on this one. So a **Forward**: a
server started in a terminal on another device of the cluster is reachable on
this device's own `localhost`, at the same port, for as long as the terminal's
tab is open here. Decided in the grilling of 2026-10-01; the vocabulary is
`CONTEXT.md`'s **Forward**, a word kept apart from the **Relay** it rides on.

**A terminal's, never the agent's session.** The session runs in the same
Sandbox and could be read the same way, but what it opens is a build cache's
port, MCP servers and language servers — none of which anybody means to open in
a browser, and every one of which would sit in the list beside the port they
did. Reading the session as well, now or behind a later switch, was rejected:
a dev server somebody wants to look at is one they start in a terminal, and a
session's that they want is one they can start there too.

**The attaching device forwards, and no other.** The device whose browser has
the terminal's tab open is the hub bridging that attach, and it alone listens.
Every device forwarding every member's ports was the other way — it is how the
merged list works — and was rejected: two members' 3000s would collide on every
device at once, and a laptop would hold listeners for a desktop's terminals
nobody on it had opened. A device with any workbench open forwarding was
rejected for the same collisions, merely coming and going with a tab. A
terminal this device's own browser attached is no Forward at all: the port is
on this machine already.

**On loopback alone, at the same number.** IPv4 and IPv6 loopback both, and no
other interface: a phone on the tailnet reaching the hub's `localhost:3000` is
a much larger thing than trying a change, with a boundary of its own to argue —
the workbench itself is reached from a phone through `tailscale serve` in front
of a loopback listener, not through an open interface.
The number is the one the server printed, because that is the URL the human
reads off the terminal. **A number already bound here is skipped and said** —
this device's own dev server, or another member's 3000 already forwarded — and
taken when it frees. Remapping to a free number was rejected, a URL the server
printed being then a URL that does not work; and the later Forward taking the
port from the earlier was rejected, a `localhost:3000` that meant one machine a
minute ago and another now being worse than one that plainly is not this.

**Read on the member, polled, while an attach over the link is held.** The
member is the only machine the terminal's processes are on, so it is the one
that reads them: a listening TCP socket, on any address and either family,
held by any process in the terminal's tree, about every two seconds — the pid
namespace bwrap made for it on Linux, the wrapper's descendants and its group
through `ps` and `lsof` on macOS, the Job Object's processes against the TCP
table on Windows. UDP is left out, there being nothing to connect. Reading
only loopback binds, or only all-interfaces ones, was rejected: a dev server
binds whichever its author liked, and either is reachable on the member's
loopback, which is all a Forward dials. The read runs while a member holds an
attach on that terminal over the link and not otherwise, the way the watcher
runs while a pane is attached; and **which member** is the certificate the
attach came in under, so the member knows whose terminals are whose without
the hub having to inspect what it bridges — the hub noting its own bridged
attaches was the other way, and was rejected for being exactly that
inspection. Where a reader cannot tell, it reads nothing and says so once in
the log — the opposite of the busy check's rule, because there the cost of a
wrong *no* is a shell ended under somebody's hands, and here it is a port not
forwarded.

**Sniffing the terminal's output alone was rejected**: VS Code reads printed
`http://localhost:3000` lines, which catches a server the moment it prints and
misses one that prints nothing. **But a printed URL hurries the read** — a
`localhost`, `127.0.0.1`, `0.0.0.0` or `[::1]` URL with a port, in the bytes
on their way to the terminal's screen, wakes the read rather than leaving it to
the next turn, so a server that says it is up is forwarded about when it says
so. **And it forwards nothing by itself.** A printed port whose listener is
outside the terminal's tree — a container's, a daemon's — forwarding where
anything on the member listens there was the other way and was rejected: the
reading is what a member will connect, below, and a line of output is not
something anybody should be able to widen that with.

**The member connects only what it reports, to a device attached to that
terminal.** Each connection accepted on the hub's listener is one upgrade over
the link, naming the terminal and the port; the member dials its own loopback
and joins the two byte for byte, and refuses by name a port not in the
terminal's reading, a terminal the caller holds no attach on, and this
device's own browser — before anything is dialled. Connecting any loopback port
a member asks for, the link being trusted, was rejected: it would make every
member a way into every other's loopback — its build cache, its workbench —
with a terminal tab as the pretext. A port that answered the reading and
refuses the dial is a refusal of the upgrade, not a socket that carries
nothing.

**Its life is the attach's, with the watcher's grace.** A Forward ends two
seconds after it leaves the reading, and leaving the reading is every ending
there is — the attach let go of, the port closing, the terminal ending — and at
once where the member reads as unreachable, there being nothing to connect to.
Ending closes the listener and every connection crossing it. Two seconds is the
watcher's, for the watcher's reason: the pane is taken down on every pane swap
and comes straight back. About thirty seconds, so that a reload kept the
connections open across it, was rejected — a tab closed on purpose would leave
a port on this machine answering for half a minute after; and none at all was
rejected, a pane swap then being a dropped connection.

**Said at the sidebar's foot, and never stopped by hand.** *Forwarding 3 ports*
stands beside *Show archived* while this device holds any Forward, forwarding
or skipped, and is not drawn at none — an item that opens onto nothing is one
nobody needs to press. A press lists each as a link to `localhost` on its port,
with the device, its OS mark and the Conversation's title; a skipped one is
dimmed and says why. That reading and its `forwards` Nudge are this device's
own and are not served over the link. No row has a control: closing the tab is
the stop. A × per row, and a × beside a switch turning forwarding off for a
device, were both rejected for this build — the Brief asked for it automatic,
and a port somebody never wants on this machine is a case nobody has met yet.

## What Code is not

Everything ADR-0013 said a terminal is not. **Not a record**: no Capture, no
Event, nothing in a Share — what the human edited is seen the way what the
agent edited is, in the Diff on the next Set and in the commits. **Not a hold
on the run**: editing a file holds nothing off, and somebody who means to take
the work over presses Stop first. **Not a session**: the terminals stay in
their own register, and the files API has no state of its own beyond the
watcher.
