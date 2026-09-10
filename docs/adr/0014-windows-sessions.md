# Windows sessions on ConPTY and an AppContainer

Windows runs sessions. [ADR-0012](0012-desktop-tray-binary.md) ported the
product there with the session machinery gated out at its leaf call sites and
one honest refusal where a session would start; this decision is the stage
after that one, the one the amendment there promised. Two things stood between
a Windows Verkstead and a session, and they are decided separately because
they land separately: the **pseudo-terminal**, which is ConPTY, and the
**Sandbox**, which is an AppContainer.

Amended (2026-09-09): **the Sandbox is a local account of Verkstead's own.** The
container was built, and the first real session inside one refused every file it
was given; three mechanisms were then asked the same questions on the same
machine and only the last of them runs the agent. The terminal half of this
decision is untouched. See *Amended: the Sandbox is an account*, and the two
*What the probe answered* sections it rests on — the title of this ADR is left
as it was written, being the record of what was decided rather than a summary of
what stands.

The order is the terminal first. A Windows session runs **unsandboxed** from
the moment the terminal works until the container lands, and the workbench says
so on every one — above **Start work** on the composer, beside the terminal on
the session pane, and on the Conversation Terminal pane — in the same voice the
"not on Windows yet" state used. That third place because a Conversation
Terminal is a shell inside the Conversation's Sandbox and on Windows there is
none: the human at that keyboard has their own account's reach, which is the
same fact the other two say about the agent, said where they will not see
either of them.
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
Data Directory and **the account's own directory** to be on one volume. The
account rather than the fresh profile: the profile is made under the Data
Directory and so is never the end that can differ, where an account is wherever
the Agent Profile points inside a Watched Path, which may well be another
drive. A machine where the two are apart refuses the session with a line saying
so. `APPDATA`, `LOCALAPPDATA` and `TEMP` point into the fresh profile too, so
npm's caches, tool state and temporary files stay out of the real one. Built in
the first stage rather than the second because it is what the container's
grants are made against, and because a session that wrote into the real profile
unsandboxed would be leaving state behind that no later stage could take back.

**A hard link is one file only while everything writes in place.** A Mac
symlinks the account into its fresh home and a symlink follows whatever happens
to the target; a file symlink on Windows needs a privilege a per-user install
has not got, which is the whole reason the link is a hard one. So an agent that
saves its config by writing a temporary file and renaming it over the top
leaves the session writing to a file of its own, with the account's copy seeing
none of it and nothing saying so. What is decided is the outcome rather than
the mechanism: **nothing a session wrote to its account is lost.** As the
session ends, a linked file that is no longer the account's own is written back
over it, and the link is made fresh for the session after. The ordinary case is
one file and costs nothing; the replacing case costs a copy rather than the
session's work. Directories are not in it — a junction is a path rather than a
file, and nothing replaces one.

## The Sandbox is an AppContainer

Amended (2026-09-09): **it is not, and this section is kept as the record of a
mechanism that did not survive the machine.** An AppContainer gives the
boundary this section describes and cannot run the agent inside it — see *What
the probe answered the second time*, below, and then *Amended: the Sandbox is an
account*, which is what the boundary is now. Everything about the reach a
session is granted survives the change almost word for word; what does not is
the identity the grants are written for and the way a process is started under
it.

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
have named. What the sweep reads them off is a record Verkstead writes under
its own Data Directory as each container is made, one file per Conversation
holding the profile's name, its SID and every entry written for it: a Closed
Conversation has no Worktree left to build a Surface from, so *the directories
the Surface would have named* has to be something the server that named them
wrote down. One profile for the installation was the alternative, cheaper per
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

### What the probe answered

*(added 2026-09-06, as the container stage was planned. The program is
`crates/server/examples/appcontainer-probe.rs`; it was run twice on a Windows 11
machine, the second time after two mistakes of its own were fixed. Everything
below is what that machine did rather than what anything expected it to do.)*

**Every claim above about the network holds, and the one this ADR called
unverified now is not.** A connection from inside a container to `127.0.0.1` is
refused, and so is one to the machine's own address on its LAN — both by
*timing out* rather than by a fast refusal, which is what a session dialling
either would experience as a hang. So the named pipe is load-bearing rather
than belt-and-braces, and binding a LAN address would never have worked.

**A pseudoconsole opened outside and handed in works.** A process started inside
a container on a console `CreatePseudoConsole` made out here printed to it and
the bytes came back off the console's own pipe. This was the claim the stage
could not have survived losing.

**A per-user process can make a profile.** `CreateAppContainerProfile` needs no
elevation, so the per-user msi install has everything it needs.

**Grants work, and the boundary is real.** A directory granted read-write is
written from inside, one granted read-only is read, and one granted nothing at
all is refused with `Access is denied`. A junction whose *target* alone is
granted is read through, so the grant belongs on the real directory the account
is at, as this ADR assumed. **And no ancestor needs granting**: the container
reached a directory deep inside the human's own profile with no entry anywhere
above it, so a rendering never has to grant the profile on the way to a
Worktree.

**Program Files and the system are readable with no entry** — `node`, `git` and
Windows PowerShell all ran inside a container that had been granted nothing at
all about them, which is what this ADR assumed and is what makes the per-user
`PATH` entries the only ones needing a grant.

**Two things came back short of an answer, and are the container stage's to
settle.** An explicit **deny** entry written under a granted tree did *not*
refuse the path beneath it — so `Nothing` over the account's own skills needs a
mechanism worked out by attempting rather than the one line this ADR imagined,
whether that is the deny ordered or flagged differently, a protected list on
that one directory, or granting the account's children rather than the account.
And **sccache does not run inside a container as it stands**: its client panicked
reading its own configuration before it ever reached the network. With loopback
refused as well, this settles the switch this ADR left to the probe — **sccache
is off for sandboxed Windows sessions**, and the shared `CARGO_HOME` stays.

### What the probe answered the second time

*(added 2026-09-08, after the first real session was run inside a container and
refused every file it was given. The probe was extended with the two questions
that session raised — whether an agent's own shell runs inside, and what node
makes of a path inside — and everything below is again what the machine did.
Nothing here is decided yet: this is what the decision above now has to answer
to.)*

**A path cannot be resolved inside a container at all.** `fs.realpathSync` and
`fs.realpathSync.native` both fail with `EPERM` on *every* path asked about —
a directory granted read-write, and `C:\Windows`, which every container reads
with no entry at all. `lstat` on those same two paths succeeds, so this is
resolution rather than reach: the JS walk starts at `C:\`, which no container
may `lstat`, and the native call asks the operating system for a handle's final
name and is refused as well. **The volume root cannot be granted**: the entries
Windows itself writes on `C:\` and `C:\Users` for isolated apps name capability
SIDs of the `S-1-15-3-65536-…` shape, and `CreateProcessW` refuses a token
built with one — *The parameter is incorrect* — while writing an entry there
for the container's own SID needs an elevation a per-user install has not got.

That is what the first session failed on. An agent that checks a path is still
what it was when permission was given calls one of those before every read, so
Claude refused its own prompt file, its own Worktree and `C:\Windows` alike,
with *its symlink resolution changed after permission was checked* — the same
refusal for a path that was perfectly granted as for one that was not.

**And an agent's shell does not start inside a container.** Git for Windows'
`bash` — which is what Claude's shell tool runs on this platform — exits
`0xc0000142`, a library refusing to start, granted or not. In a session it says
what it is:
`NtCreateDirectoryObject(\BaseNamedObjects\msys-2.0S5-…): 0xC0000022`.
msys2 makes its shared objects under the machine's own `\BaseNamedObjects`,
which an AppContainer is refused, so this is msys2 and the boundary rather than
anything a grant reaches.

**What does work is everything else this probe had already asked.** `node`,
`git`, `pwsh` and Windows PowerShell all still run inside; a batch file in a
granted directory runs through `cmd /d /c call` exactly as
[`sandbox::open`](../../crates/server/src/sandbox/open.rs) runs one, so an
npm-installed `claude.cmd` starts. The `npm` line's own failure is npm's script
rather than the shell. PowerShell inside prints
*InitializeDefaultDrives … failed* on every start, which is the same denied
volume root seen from a different program.

### And what the two probes after it answered

*(added 2026-09-09. The container having failed, the two mechanisms this ADR
had set aside were asked the same questions the same way —
`crates/server/examples/restricted-token-probe.rs` and
`crates/server/examples/session-account-probe.rs`. Again, everything here is
what the machine did.)*

**A restricted token gives the boundary and breaks the toolchain, three ways
out of three.** A restricted SID list of the session's identity alone will not
start a process at all; one holding `Everyone`, `Users` and `RESTRICTED` beside
it starts a process, reaches what it is granted, refuses what it is not, and
refuses the human's profile — and under it node dies initialising its random
source, both PowerShells fail to load, and msys2 fails querying its own token.
Widening the list by this logon's own SIDs changed nothing; writing entries on
the token, the window station and the desktop changed nothing; the human's own
SID turned **deny-only** gave the best boundary of the three — their profile
refused to reads as well as writes — and broke the same three programs and the
registry with them.

**Only the integrity level leaves the toolchain standing, and it costs bash.**
A token lowered to low integrity, with no list at all, runs node, both
PowerShells and git; resolves paths; carries a console; reaches loopback; and a
write lands only where Verkstead has written a mandatory label and is refused
everywhere else including the human's own files. Their files stay *readable*,
which is the weaker promise. And `bash` will not start: msys2 makes its shared
objects under `\BaseNamedObjects`, which is labelled medium, and nothing
standing lower may write there. Medium-low integrity behaves identically.

**What all four have in common** is that the process is not quite the human — a
stranger's identity, a second access check, a deny-only account, a ceiling —
and node, msys2 and a managed runtime each need it to be an ordinary one.

**A local account of Verkstead's own is an ordinary one, and everything works.**
A directory granted to it is written and read, one not granted is refused, a
junction is followed to its target, the human's own profile is refused to reads
*and* writes, loopback connects, and `node`, `git`, Windows PowerShell and
**`bash`** all run. Three things came with it:

- **A console cannot be handed to a process started as another account.**
  `CreateProcessWithLogonW` — the one call that starts a process as somebody
  else without a privilege a per-user install has not got — refuses an extended
  startup info with *The parameter is incorrect*. What works is a **launcher**:
  a program of Verkstead's own, started as the account with the two pipes as its
  plain standard handles, which calls `CreatePseudoConsole` over those and
  starts the session on it with an ordinary `CreateProcessW`. The marker came
  back off that console. Nothing is duplicated across the boundary — the
  handles the console needs are the ones the launcher inherits.
- **Ancestors need granting after all**, which reverses what the first probe
  found for a container. *Reaching* a deep path needs no entry above it, because
  an ordinary account holds the privilege to skip the traverse check — but
  *resolving* one walks the prefixes and asks each for its attributes, and the
  human's profile directory is on that walk. One `FILE_GENERIC_EXECUTE` entry,
  not inherited, on each directory along the way is the whole of the fix, and it
  says nothing about what is inside: the profile stayed unlistable in the same
  run that resolved through it.
- **`pwsh` is refused with error 1920.** It is a Store execution alias and those
  are per-user, so the session account cannot resolve the human's. Windows
  PowerShell runs, which is the fallback this ADR already names for a
  Conversation Terminal — now the ordinary case rather than the exception.

## Amended: the Sandbox is an account

*(2026-09-09. This replaces *The Sandbox is an AppContainer* above, which is
kept as the record of what was tried.)*

**A Windows session runs as a local account of Verkstead's own.** It is still
the Mac's kind of boundary — the machine is there and refused — and still one
description rendered onto real paths: every path the Surface names is granted to
that account's SID at the reach the description says, and what is not granted is
not reachable. What changes is who the process is. An AppContainer identity, a
restricted SID list, a deny-only SID and an integrity ceiling were each tried
and each broke the agent; an ordinary account breaks nothing, because from the
machine's point of view there is nothing unusual about it.

**Which costs an elevated step, once, and this ADR reverses itself to pay it.**
The loopback exemption was refused for being *one elevated command per machine,
which the per-user install has no way to run* — and that refusal bought a named
pipe, which was cheap. Refusing it here costs the shell, the human's files being
unreadable, and per-Conversation isolation together, which is not. So the
installer asks for elevation once and creates the account; everything after that
is unprivileged, and a machine where the step was declined has no Windows
sandbox and is told so in the words the unsandboxed note already uses.

**One account for the installation, not one per Conversation** — reversing Q14
along with it, and for the reason that makes it unavoidable: creating an account
needs elevation, so one per Conversation would need elevation per Conversation.
What that costs is what Q14 bought: a session can reach every *live*
Conversation's Worktree, because they are all granted to the one account. What
it does not cost is the boundary the Sandbox exists for — the human's own
machine, their profile, their repositories outside the Watched Paths and their
account's own skills are all still refused. The entries still go when a
Conversation's Worktree does, so the reach is what is granted now rather than
everything ever granted, and the record under the Data Directory still says what
was written so a server that died can take it back.

**The password is Verkstead's to keep.** `CreateProcessWithLogonW` needs one and
there is no passwordless route to another account's token without a privilege
only the operating system holds, so the installer generates a long random one
and it is kept beside the other secrets under the Data Directory. The account is
made with a password that does not expire and that it cannot change, and is
denied interactive logon: it is a name to run as rather than one anybody signs
in with.

**The launcher is a verb of Verkstead's own binary**, which a session already
has bound in read-only — see [`Executable`](../../crates/server/src/sandbox.rs).
It makes the pseudoconsole, starts the agent on it, and is what the Job Object
holds, so a server that dies still takes its sessions with it and an ended
session still takes everything it started. Its standard handles are the
console's two ends; resizing reaches it over the named pipe this ADR already
has, whose descriptor grants the session account the way it was to have granted
the container.

**And two things the container had to do without come back.** Loopback works
from the account, so `verkstead ask` would reach the TCP socket — the named
pipe stays all the same, being landed, harmless and the one transport that
needs no firewall to agree with it. And **sccache is back on**: its client
reaches the Compile Server, so the switch this ADR left to the probe falls the
other way from the container's answer.

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
