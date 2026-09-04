# Windows sessions on ConPTY and an AppContainer

Windows runs sessions. [ADR-0012](0012-desktop-tray-binary.md) ported the
product there with the session machinery gated out at its leaf call sites and
one honest refusal where a session would start; this decision is the stage
after that one, the one the amendment there promised. Two things stood between
a Windows Verkstead and a session, and they are decided separately because
they land separately: the **pseudo-terminal**, which is ConPTY, and the
**Sandbox**, which is an AppContainer.

The order is the terminal first. A Windows session runs **unsandboxed** from
the moment the terminal works until the container lands, and the workbench says
so on every one — above **Start work** on the composer and beside the terminal
on the session pane — in the same voice the "not on Windows yet" state used.
Chosen over holding sessions back until the container is there, for the reason
ADR-0012 chose a clear notice over waiting for the ports: the container's
unknowns can only be settled on a Windows machine, and a terminal held hostage
to them might ship nothing. What the note says is the plain truth of it — the
agent runs with the human's own account's reach — and the note goes the day the
container arrives. Not behind an opt-in setting, either: a setting would be a
second place to say what the note already says.

## The terminal

The Windows arm of [`terminal`](../../crates/server/src/terminal.rs) is a
ConPTY, opened with `CreatePseudoConsole` and sized with
`ResizePseudoConsole`, and the two pipes it hands back are what the relay
reads and what a keystroke is written into. Written by hand against
`windows-sys`, which the desktop crate already depends on, the way the Unix arm
is written against `rustix` rather than a PTY crate.

**The process is spawned by hand as well**, with `CreateProcessW` and an
attribute list carrying the pseudoconsole. Rust's `Command` cannot attach one —
the extension that would is still unstable — so the Windows arm returns a
`Child` of its own rather than tokio's: a process handle inside a **Job
Object** configured to kill everything in it when the last handle closes. That
Job is what `--die-with-parent` is on Linux and the keeper process is on a Mac
(see `sandbox::outliving`): a server that dies takes its sessions with it, and
an ended session takes every process it started. What the rest of the sessions
module asks of a child — its id, its exit, a kill — is the same on both arms.

The agent is found on `PATH` with `PATHEXT`, so an npm-installed `claude.cmd`
starts as well as the native installer's `claude.exe`, and the command line is
quoted by the rules `CommandLineToArgvW` reads it back with. Environment is
cleared and set explicitly, as every rendering does, with the Windows names
added that nothing runs without — `SystemRoot`, `ComSpec`, `PATHEXT`, `TEMP`,
the profile roots. `PATH` inside is Verkstead's own bin directory followed by
the server's own `PATH`: there is no fixed machine `PATH` to name, the way
`LINUX_PATH` and `APPLE_PATH` name one, because where tools live on a Windows
machine is where the human put them.

**The prompt goes to a file on Windows.** A session's prompt is one argument
today, and an implementing session's carries the handoff document inlined;
Windows caps a command line at 32,767 characters, which a long handoff
exceeds. So on Windows the prompt is always written to a file in the
Conversation's handoff directory and the agent is started on one line naming
it — always rather than only when it would not fit, so that a Windows session
has one shape rather than two, and only on Windows, because nothing on the
other platforms is the worse for the argument. `nix develop` is skipped there
by Platform rather than shelled out to: there is no nix on Windows, and a
session should not pay for finding out.

**Conversation Terminals** ([ADR-0013](0013-conversation-terminals.md)) open on
the same ConPTY. There is no passwd entry to read a login shell from, so a
Windows terminal opens `pwsh` where it is installed and Windows PowerShell
where it is not.

## The fresh profile

A session on Linux starts in an empty `HOME` with only the account mounted
in; on a Mac the same, in a directory made fresh under the Data Directory. A
Windows session gets the same shape from the first stage, container or no
container: a directory of its own under the Data Directory that `USERPROFILE`
and `HOME` point at, with the Profile's account joined into it.

**The rule is over the account rather than over one backend.** Four agent types
keep an account four ways — Claude's pair at `.claude` and `.claude.json`,
Codex's `.codex`, Grok's `.grok`, opencode's config and data directories — and
the Surface already binds whichever of them the Profile names, so a rule
written for Claude's pair would start a Codex or opencode session into a
profile with no account in it at all: logged out, and nothing saying why. So
every **directory** in the account is joined in by a **directory junction**,
which needs no privilege, and every **file** by a hard link, which needs the
Data Directory and the profile to be on one volume — a machine where they are
not refuses the session with a line saying so. `APPDATA`,
`LOCALAPPDATA` and `TEMP` point into the fresh profile too, so npm's caches,
tool state and temporary files stay out of the real one. Built in the first
stage rather than the second because it is what the container's grants are
made against, and because a session that wrote into the real profile
unsandboxed would be leaving state behind that no later stage could take back.

## The Sandbox is an AppContainer

What a session may reach is one description rendered three times now:
bubblewrap's flags on Linux, a seatbelt policy on a Mac, and on Windows an
**AppContainer** — the platform's own deny-by-default boundary, the one its
browsers run their renderers in. It is the Mac's kind of boundary rather than
Linux's: **the machine is there and refused**, not absent. Every path in the
Surface that Linux would bind is a real path, and reach is an access-control
entry on it granting the container's identity: the Worktree and the Repo's git
directory read-write, the account, the handoff directory, Verkstead's own bin
and the Skills read-only, and — because a per-user install of a tool is not
readable by a container the way Program Files is — **each `PATH` entry under
the human's profile read-only**, and nothing else of the profile. `Nothing`
over the account's own skills is an explicit deny entry, which is what a
rendering with no mount to hide a path with does (the seatbelt's `require-not`
is the same move). Program Files and Windows are readable by every container
already and need no entry.

**One profile per Conversation.** Grants are entries on the human's real
directories, so what they reach and when they go matters: a Conversation's
profile is granted at its first session and removed with its Worktree, so a
session reaches its own Worktree and its own binds and no other Conversation's.
A server that crashed between the two would leave entries behind, so **the
server sweeps at startup**: profiles of Conversations that are Done or Closed
are deleted and their entries stripped from the directories the Surface would
have named. One profile for the installation was the alternative, cheaper per
session and with nothing to undo, and was rejected for letting any session
reach every directory ever granted.

**Temporary files** are a directory of the session's own inside the fresh
profile, removed when the session ends — Linux's tmpfs rather than the Mac's
shared `/tmp`, because on Windows nothing reaches for a literal `/tmp` and
`TEMP` is already a variable. **Network** is the internet-client capability
and nothing else: the filesystem is the boundary and the network is not, as
everywhere, and the private-network capability would open the LAN for nothing
a session needs. **A container that cannot be made** — a profile that will not
create, a grant that fails — refuses the session, the way a missing `bwrap`
does on Linux; it never falls back to the unsandboxed session of the first
stage, whose note would then be a lie.

### Loopback, and the named pipe

An AppContainer is refused connections to the local machine, and the exemption
that lifts it is an elevated command per machine, which an unsigned per-user
install cannot ask for. That reaches the one thing every session does:
`verkstead ask` at `127.0.0.1:8422`. So **the server listens on a named pipe
beside its TCP socket**, the pipe's security descriptor grants the container,
`VERKSTEAD_SERVER` inside a Windows session names the pipe, and the CLI's
`--server` takes a pipe as well as a URL through a transport of its own under
ureq's `Connector`. Binding a tailnet or LAN address as well was considered
and rejected: whether the firewall counts a machine's own address as loopback
is unverified, and a session would depend on an interface existing. The pipe
is its own stage, before the container: it changes the CLI's public surface,
and it can be proved on every platform's tests with no container at all.

The same block reaches sccache, whose client talks to the Compile Server over
loopback TCP. The shared `CARGO_HOME` is directories and works regardless;
whether sccache stays on for sandboxed Windows sessions is what the probe
below decides, and it is off where the probe says loopback blocks it too.

### The probe comes first

Three claims here are made from documentation rather than from a machine: that
a desktop AppContainer is refused loopback, that node and `pwsh` run under one
at all, and that a ConPTY works inside one. The container stage therefore
**opens with a probe** — a small program the human runs on their Windows 11
machine and pastes the output of — before the rendering is written, and the
stage's own grilling reads that output. A rendering built on a claim the
machine contradicts would be a stage rebuilt.

## What stays as it was

Linux and macOS sessions are untouched: no prompt file, no fresh-profile
change, no transport change. The headless daemon, the nix flake and the NixOS
module are unchanged. `SessionsHere::NotOnWindowsYet` and the wording it
carries go with the first stage, replaced by the unsandboxed note; the note
goes with the third.

## Considered Options

- **Sessions in WSL2, running bubblewrap** — reuses the Linux rendering whole,
  and rejected: the repository on `/mnt/c` is slow, the tools live in the
  distro rather than on the machine, every path has two spellings, and a
  human who has WSL can run the Linux Verkstead in it today.
- **A restricted token at low integrity** — writes refused outside grants, but
  the whole machine readable; weaker than either rendering that exists, and
  the leak is the one the Sandbox exists to close.
- **No sandbox on Windows, said plainly and permanently** — honest, and
  rejected for making the product's promise one platform short.
- **A loopback exemption for a fixed profile SID** — one elevated command per
  machine, which the per-user install has no way to run and the docs would
  have to ask for.
- **The `portable-pty` crate** — mature and maintained, and rejected for the
  reason the Unix arm is not a PTY crate: the surface needed is small, and a
  hand-written arm can be read beside the one it mirrors.
- **The prompt file on every platform** — one behaviour everywhere, rejected
  for changing what Linux and Mac sessions see for no reason of their own.
