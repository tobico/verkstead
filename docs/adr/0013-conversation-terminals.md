# A human's terminal in the Conversation's sandbox

Amends [ADR-0007](0007-server-held-terminal.md): the server-held terminal is
no longer only a session's.

A Conversation can open terminals of its own — a **Terminal** button beside
Share on the Timeline's head, a details pane at `/terminal`, a tab bar of
shells — for the moment the agent's work is done and the human wants to try
it, make a small change or work with git without leaving the workbench. Each
tab is a pseudo-terminal Verkstead allocates and holds, feeding a server-side
virtual terminal that the browser attaches to over a websocket with xterm.js
as the window, exactly as ADR-0007 built for a session's Screen. What runs on
it is a shell rather than an agent, and that is the whole of the difference
at the terminal layer: the same repaint on attach, the same raw bytes after,
the same keystrokes and resizes up, one screen for every watcher and the
latest resize sets the size.

## What runs, and where

The shell runs **inside the Conversation's Sandbox** — the one
`Sandbox::for_conversation` builds for the implementation pairing's Profile,
the grilling pairing's where the implementation role has none — with a
session's whole environment: the Worktree as the working directory, the git
directory, the handoff directory, the files the human attached, the build
cache, the Sandbox Configuration binds, the GitHub token, the git author and a
`VERKSTEAD_SERVER` scoped to the Conversation, wrapped in the worktree's nix
dev shell where its flake has one. A terminal has no role of its own, and the
implementation Profile is the account the work is done under, so `claude`, `gh`
and `git` behave in a terminal as they do for the agent. Running it outside the
Sandbox was never on the table: the filesystem boundary is what makes a shell
in a Conversation safe to offer at all, and a terminal that could reach the
checkout the Worktree was made from would be the one thing in Verkstead that
could.

It runs the **server user's login shell** from passwd, `/bin/sh` where that is
missing, not inside the Sandbox, or a system user's `nologin`. No setting: the
shell a human gets at the machine is the shell they expect, and the packaged
install's answer is the nix module giving its service user a shell
(`services.verkstead.shell`, bash by default) rather than a second place to
configure one. Interactive and not a login shell, because a login shell reads
the system profile, which rebuilds `PATH`, and the Sandbox's invariant that
the running server's own `verkstead` is first on `PATH` has to hold here too.

Not being a login shell turned out not to be enough on the machine this is
built on. Every shell on NixOS — `/etc/bashrc`, `/etc/zshenv`, fish's own
preinit — sources `/etc/set-environment` unless
`__NIXOS_SET_ENVIRONMENT_DONE` is already set, and that file rebuilds `PATH`
out of the *host's* profiles: a terminal's `verkstead ask` would have run
whatever the machine had installed rather than the server's own image. So a
Sandbox built to run a shell says that variable, which is the plain truth of
it — the environment was set, here, by the Sandbox — and off NixOS it is a
variable nothing reads.

## What a terminal is not

**Not a record.** A terminal is memory only: no Capture, no Event on the
Timeline, nothing in a Share, and gone when the server stops. A session's
bytes are kept because they are the record of what an agent did; a human's
shell is the human doing something, and the workbench records what happened
to the Conversation, not what its human typed. Capturing the bytes was
considered and rejected as a record nobody asked for at the cost of a store
table and a replay path.

**Not a hold on the run.** Typing into a terminal holds nothing off, exactly
as typing into a Screen does not (ADR-0007, and the Screen's vocabulary
entry): no badge, no Event, and Stop is the human's move where they mean to
take the work over. Stopping the run when a terminal opens was rejected —
most terminals are opened after the work is done, when there is nothing to
stop, and a terminal opened beside a running session is a human looking, not
a human taking over.

**Not a session.** A Conversation may have several at once, so they live in a
register of their own rather than a bend in the one-session-per-Conversation
map, keyed by a number issued in order and never reused. They end when the
shell exits, when their tab is closed, when the Conversation closes (before its
Worktree is removed) or when the server stops — and for no other reason. An
idle reaper and ending on leaving the pane were both rejected: the server
holds the terminal so that closing the pane, switching devices or losing a
connection loses nothing, and a reaper would take back with one hand what
that gives with the other.

## Auth

There is none of its own, as there is none for the Screen's socket: the
tailnet is the perimeter. This is a larger grant than the Screen's — an
arbitrary shell rather than keystrokes into an agent that already has one —
but not a larger one than Verkstead already makes: every Screen attach can
type into an agent whose shell tool runs the same commands in the same
Sandbox, and a session's Sandbox is what bounds both. What the terminal gets
that the Screen has is the size checked on the way in, since it arrives from
outside.

## Scrollback

The virtual terminal holds the grid alone, and a fresh attach starts from it.
xterm keeps scrollback for as long as a tab is attached, so a long test run
can be read back on the device that ran it, and a reload or a second device
starts at the grid. Server-held scrollback per terminal — a new repaint shape
and memory per tab — was considered and set aside as more than the use needs.

## The tab rules

The pane never stands empty: one terminal opens when the pane loads with none
live, a tab goes when its shell ends, and when the last goes while the pane
is open a new one opens. The one guard on that is against a shell that cannot
start: a tab whose shell ended within five seconds of being asked for, or
whose open was refused, stays showing why and nothing opens until the human
presses plus — otherwise a refused Sandbox would be an endless spawn loop.
Close is on a context menu, right-click on a pointer and long press on a
finger, so a close cannot be pressed by accident.

A tab is called what its shell calls itself: xterm reads the terminal title
escape, and the tab's label follows it, most prompts setting one at every
prompt. Where the shell has set none, or has cleared the one it set, the label
is *Terminal N* by the number the server issued — which is why those numbers are
never reused. A repaint carries the grid and not the title, so a fresh attach
reads the number again until the shell next says a name.
