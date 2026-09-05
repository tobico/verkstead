# Adoption

The switch-over: what Verkstead replaces, and how a day's work runs through it.
Working *on* Verkstead is [development.md](development.md); this is working
*with* it.

Until this point the loop was three tools kept in step by hand — askance for
the questions, `tobico-skills/roadrunner` for the driving, and the
`tobico-scripts` wrappers for the sandbox. They are what built Verkstead, stage
by stage, and they are what it replaces. Starting a piece of work stops meaning
*which of the three does this part* and starts meaning *open the workbench*.

**Where this stands.** The switch-over has been made for this repository:
Verkstead drives its own work, and it is what executes this roadmap — a Stage
of it is a Conversation, planned and built by sessions Verkstead starts. What
has *not* happened yet is a Conversation reaching **Done**. Every run so far
has been driven by hand past the finish: the step that pushes and opens the
draft pull request has never carried a Conversation into Wrapping, so the
wrap-up loop and the settling below it are still proved by the test suite
alone. Closing that is [stage 05](roadmaps/mvp/05-refinement.md), which is
being built the way everything else here is — by Verkstead, through this page.

The vocabulary in bold is the project's, defined once in
[CONTEXT.md](../CONTEXT.md).

## What it replaces

| Before | Now |
| --- | --- |
| `sandbox` / `work-sandbox` — bwrap around the whole of `~/src` | A **Sandbox** per **Conversation**: its **Worktree**, its Repo's git directory, its handoff directory, the **Agent Profile**'s claude pair, and nothing else of the machine |
| `agent`, `grilling`, `next-stage`, `next-tasks` — one wrapper per thing you might start | One **Conversation**, which runs through Draft → Grilling → Direction → Implementing → Wrapping → Done |
| `roadrunner` — a terminal per run, driving `.tasks/` and `docs/roadmaps/` | The orchestrator, driving the same two files off the Repo, with the run visible on a **Timeline** instead of scrolling past |
| roadrunner's interruptions | A **Halt** and its stop **Notice** — pushed to your phone, read where the work is, and answered by one **Resume** |
| askance — one queue of Question Sets for the machine | **Question Sets** on the Timeline of the Conversation they were asked from |
| The skills installed under `~/.claude/skills` | **Skills** shipped inside the binary and read-only inside at a path no backend owns, with the account's own not reachable at all, so a session's behaviour is the product's |
| A gate at every commit | No commit gates. Review consolidates in the wrap-up, per pull request |

What stays: **askance is a separate, maintained product**, and the
`tobico-skills` skills stay installed for ordinary terminal work in
repositories Verkstead is not driving. What retires is roadrunner and the
wrappers that launched it — see [The old tools](#the-old-tools).

## Getting it running

Nothing has been released under this name yet, so what follows is what a `v*`
tag produces rather than something to fetch today — [releasing.md](releasing.md)
says what a tag builds and where it puts it.

There are four ways in, and they are four different things rather than four
spellings of one. **The flake and the NixOS module run the headless daemon**, on
a machine that is always on and answering from wherever you are. **The AppImage
is the same server started from an icon**, on the Linux desktop in front of you,
with the viewer in your browser and a tray icon over it. **The dmg is that same
app for a Mac**, with the icon in the menu bar instead. **The msi installs that
same app on Windows**, into your own profile and without asking for
administrator. Which one you want is which of those machines you were
describing; two at once is two Verksteads, and the second to reach port 8422
says so in a dialog and exits.

### The daemon, on NixOS

On a NixOS host, import the flake and enable the service:

```nix
services.verkstead = {
  enable = true;
  watchedPaths = [ "/home/you/src" ];
  home = "/home/you";                 # optional; the service's own by default
  sandboxBinds = [ "verkstead=/var/cache/verkstead-node" ];
};
```

Three of those are worth understanding before the first Conversation, because
each is a boundary rather than a convenience:

- **`watchedPaths`** is what Verkstead may operate inside. There is no default
  and no scan; a **Repo** is registered only from within one, and a path that
  merely reads as inside one is refused. The module refuses to build with none:
  the server itself would start, because the settings page says Watched Paths
  too, but a directory this unit was never told about is one its hardened
  namespace does not hold — so on NixOS this is the list that counts.
- **`home`** is only what `HOME` means for the service; nothing is read out of
  it and nothing of it reaches a Sandbox. Credentials and identity are said
  instead: a token in `secrets.yaml` and a `git_author` in `config.yaml`, both
  in the data directory, reaching each session as `GH_TOKEN` and git's own
  `GIT_CONFIG_*`.
- **`sandboxBinds`** is the **Sandbox Configuration** — every entry is a hole
  in the boundary, which is why one that is not there refuses startup rather
  than being skipped. A bare path goes to every session; `name=path` goes only
  to sessions working in the Repo registered under that name.

The workbench says both lists as well, on the settings page's **Paths** section
and on each Repo's own pane, and what a session gets is the union of the two.
Those entries are saved into `config.yaml` in the data directory, read afresh
every time they are used, and never fatal: one the server cannot see is reported
on the page rather than refused, and simply covers nothing. **On this module,
that report is the one to read** — the unit's namespace holds what the options
above name and nothing else, so a path typed into the settings page saves, says
the server cannot see it, and does nothing until it is added here too. A bare
binary outside NixOS has no such namespace and needs no flags at all: see
[development.md](development.md#quickstart).

A Rust build cache is not one of them, and there is nothing to configure for
one. The **Build Cache** is the server's own: the module makes
`/var/cache/verkstead`, puts `sccache` on the service's path, and every Sandbox
gets the directory writable with `CARGO_HOME` inside it and `sccache` as its
`RUSTC_WRAPPER` — so a crate is downloaded once and compiled once for the
machine rather than once per Conversation. The sccache server every Sandbox
compiles through is Verkstead's own, in a Sandbox of its own holding the
worktrees and the cache, and it comes and goes with the service. Whether
Sandboxes get one, and how
large its compiled half may grow, are in the workbench settings; it is on with
nothing configured. `systemctl clean --what=cache verkstead` empties it, and
nothing but build output is in it.

The **Data Directory** is not one of the three either, and not a choice on this
module: the unit keeps it in its own state directory, `/var/lib/verkstead`, and
passes it as `--data-dir`. Started without that flag — the same package run by
hand — Verkstead keeps it in the platform's own place instead:
`~/.local/share/verkstead` on Linux, `~/Library/Application Support/Verkstead`
on macOS, `%APPDATA%\Verkstead` on Windows. One directory either way, holding
the database, the Worktrees, the Skills, the handoff directories and both
settings files.

The server binds loopback and speaks plain HTTP. Answering from a phone needs
HTTPS, which is `tailscale serve --bg 8422` in front of it — and push
notifications need that HTTPS to work at all.

### The desktop app, on a Linux machine

`Verkstead-x86_64.AppImage` is one file holding the server, the viewer and every
library the tray is drawn over, so a machine with none of them installed needs
nothing else to draw a tray icon. x86_64 only: an arm64 Linux desktop has the
bare CLI and `verkstead serve`. Downloaded, made executable — a Release asset
carries no mode — and run, it serves on `127.0.0.1:8422`, opens the viewer in the default
browser, and puts an icon in the tray with the four things a browser tab cannot
do for itself: **Open** brings the viewer back, **View Logs** opens the file the
server's logging goes to when there is no terminal to print it in, **Launch on
Startup** is a checkbox over the desktop's own startup registration, and
**Exit** stops the server. `--no-open` starts it without the browser, and
`--data-dir` moves the Data Directory off `~/.local/share/verkstead`.

**What is inside is the whole `verkstead`**, and the icon is one verb of it:
the entry point in the file runs `verkstead desktop`, because a desktop
launcher names a file and has nowhere to say a verb — which is also why the
flags above are the app's rather than the CLI's. The same binary is what a
session started here is handed to ask with, so the two halves of an ask are
one build ([ADR-0012](adr/0012-desktop-tray-binary.md), as amended) — and the
libraries it was packed with go in beside it, so a session can run it on the
machine this file was made for as surely as you can.

**A desktop with no tray host shows no icon, and nothing is wrong.** Vanilla
GNOME is the case people meet — it draws no tray, and an AppIndicator extension
is what gives it one. Verkstead cannot tell that from a tray that is drawing the
icon, because the appindicator registers on the bus either way, so there is no
message it could honestly give you. What it does instead is what it does
everywhere: serve, and open the viewer. The viewer is the whole interface — the
tray holds those four items and nothing else — so what is lost is the icon
rather than the app: the viewer is a URL you already have, the log file is in
the platform's log directory, and stopping it is stopping the process. The
extension is what gets the four back, and there is nothing to reinstall or
reconfigure here once it is on.

**Three things stay the machine's**, and a bundle is the wrong place for any of
them.

**Sessions need bubblewrap**, and it cannot ride inside: an AppImage is mounted
`nosuid` and its files sit at a path made for one run, so a copy carried in the
bundle would be denied the privilege bwrap needs however it was granted. The
NixOS module puts it on the service's path; a desktop elsewhere wants the
distribution's `bubblewrap` package installed.

**The C library is the host's**, because a process holding two of them has two
of everything a C library keeps. The bundle is built against glibc 2.35, which
is the floor it runs on: Ubuntu 22.04, Debian 12 and anything newer will load
it, and a distribution older than those — RHEL 9 and its family among them —
will not, saying `GLIBC_2.35 not found` and nothing friendlier.

**And FUSE, because an AppImage mounts itself.** It wants a `fusermount` on the
`PATH` and a `/dev/fuse` to open; every desktop install has both, and a minimal
or hardened one may not. Without them the file says so — "Cannot mount AppImage,
please check your FUSE setup" — and `--appimage-extract-and-run` is the way past
it for a machine you cannot change.

### The desktop app, on a Mac

`Verkstead-universal.dmg` holds `Verkstead.app`: the same server and the same
viewer the AppImage carries, drawn over AppKit instead of GTK, and universal —
the Apple silicon build and the Intel one are in the one executable, so there is
one download and no architecture to choose between. macOS 11 is the oldest it
will start on. Open the image and drag Verkstead into the Applications folder
beside it in the window, which is the whole of the install.

Inside the bundle is the whole `verkstead`, with a small launcher script beside
it that supplies the `desktop` verb — a bundle names an executable and has
nowhere to say a verb — so the app you double-click and the binary a session
asks with are one build ([ADR-0012](adr/0012-desktop-tray-binary.md), as
amended).

**The first launch is then refused, and that is expected.** The app is unsigned
— there is no Developer ID behind it, which is
[ADR-0012](adr/0012-desktop-tray-binary.md)'s decision rather than an oversight
— and Gatekeeper will not open an app that arrived over the internet unsigned
just because somebody double-clicked it. What it says is that macOS "could not
verify" Verkstead "is free of malware", in a dialog with no way past on it.
There is a way past, and it is three steps — the first of them being the launch
that fails, because the refusal is what puts Verkstead in the list the second
step reads:

1. Double-click **Verkstead** in Applications, and click **Done** on the
   refusal.
2. Open **System Settings → Privacy & Security** and scroll down to
   **Security**. `"Verkstead" was blocked to protect your Mac` is there with an
   **Open Anyway** button beside it: click it, and authenticate.
3. macOS asks once more, in a dialog that this time has an **Open Anyway** on
   it. Click that, and the app starts.

Once, rather than at every launch: what was approved is that copy of the app,
and starting it afterwards — by hand, or from Launch on Startup — is ordinary.
Replacing it with a newer download is a different copy and wants the same three
steps again.

What is on the screen after that is an icon in the menu bar, and the menu on it
is the Linux tray's four: **Open** brings the viewer back, and heads the menu a
click on the icon opens; **View Logs** opens the file under
`~/Library/Logs/Verkstead` that the server's logging goes to when there is no
terminal to print it in; **Launch on Startup** is a checkbox over a launch
agent at `~/Library/LaunchAgents/net.tobico.Verkstead.plist`; and **Exit**
stops the server. `--no-open` starts it without the browser and `--data-dir`
moves the Data Directory off `~/Library/Application Support/Verkstead`, both of
them for a run from a terminal — an app launched from Finder is launched with no
arguments at all.

macOS keeps a **Login Items** list of its own beside that plist, in a database
the file is not in, and the checkbox cannot see it: switching Verkstead off
there leaves the box ticked and the plist where it was.

**Sessions run on a Mac**, and what one may reach is the same description as on
Linux rendered over Apple's sandbox instead of bubblewrap: the Conversation's
Worktree, the Repo's git directory and the handoff directory writable, each
Companion Repo at the mode it was set to, the Sandbox Configuration's entries,
the Build Cache with the machine's one `sccache` behind it, a HOME of the
session's own with the Agent Profile's account inside it, the Skills and the
`verkstead` a session asks with read-only, the system read-only, `/tmp`, the
network whole and unfiltered, and nothing else of the machine.

**`/tmp` is the one place a Mac session reaches more than a Linux one**, and
the one thing on that list that is not the same on both. On Linux it is a
filesystem of the session's own: it holds nothing of the machine's, and it goes
when the session does. A policy has nothing like that to offer, so on a Mac it
is your real `/tmp` — a session can read whatever else on the machine left
something there, and what it writes stays behind for whoever looks. That is
deliberate rather than an oversight: giving a session a temporary directory of
its own would mean refusing every tool that reaches for the literal `/tmp`,
which is most of them. Nothing of Verkstead's is kept there — the handoff
document a grilling writes goes under the session's own HOME on a Mac, so two
Conversations running at once are not writing to one path.

**What differs is that the boundary refuses rather than hides**, and it is worth
knowing which of the two you have. A session on Linux is in a namespace your
home directory was never in; a session on a Mac is looking at a machine that has
one, and is refused every byte of it. What it can still read is the metadata: a
path it may not open answers `stat` and then refuses to open, because that is
what a Mac looks like from inside a policy and a rule per path to pretend
otherwise would buy nothing. And what a mount makes out of nothing is made for
real instead: the session's HOME, the account linked into it, and the directory
holding the Skills and the `verkstead` binary are all really there under the
Data Directory, and what keeps one Conversation out of another's is the policy
rather than the absence.

**Nothing outlives the app.** Exit off the menu is a stop where it stands, as it
is on Linux, and so is the process being killed outright: every session and the
compile server go with it either way. Linux has that from bubblewrap's
`--die-with-parent`; a Mac has no such flag, so Verkstead starts a keeper beside
each sandbox whose whole job is to end it once the server is gone.

**Two things stay the machine's**, where three do on Linux.

**The tools a session runs**, because the bundle is the server and the viewer
and not a toolchain. `git`, `node`, `cargo` and whatever else an agent reaches
for are the Mac's own: Apple's under `/usr/bin`, where `git` arrives with the
Xcode Command Line Tools; Homebrew's under `/opt/homebrew`; and nix's under
`/run/current-system` where the Mac is running nix-darwin. All three are on a
session's `PATH` inside and readable through the boundary, and a Mac with none
of them installed has sessions that can run a shell and not much else.

**And the sandbox itself**, which is `/usr/bin/sandbox-exec`: on every Mac,
nothing to install, and deprecated by Apple with no replacement an unsigned app
can use — the supported way to sandbox is an entitlement on a signed bundle,
applied to the app itself rather than to a child it spawns. ADR-0012 takes that
with open eyes, and it is the one thing here that could stop working without
anybody touching Verkstead: the day the command goes, Mac sessions go with it
until something replaces them.

### The desktop app, on Windows

`Verkstead-x86_64.msi` is the download, and it is an installer rather than a
file to keep wherever you like: a Windows install is two files — the whole
`verkstead`, and a small windows-subsystem shim beside it that supplies the
`desktop` verb a Start-menu shortcut has nowhere to write
([ADR-0012](adr/0012-desktop-tray-binary.md), as amended) — and two files
beside each other are not a portable download. x86_64 only, which is every
Intel and AMD machine, and an arm64 one runs it under the emulation Windows
does for exactly this.

**Opening it is stopped the first time, and that is expected.** The package is
unsigned — there is no code-signing certificate behind it, which is
[ADR-0012](adr/0012-desktop-tray-binary.md)'s decision rather than an oversight
— and Windows marks a file that arrived from the internet, so SmartScreen puts
a blue **Windows protected your PC** window in front of it with a **Don't run**
button and nothing else that looks like a way on. There is a way on, and it is
two clicks:

1. Click **More info**, which is the line under the message and the whole of
   what is hidden here.
2. It names the file and says *Unknown publisher*, and a **Run anyway** button
   appears at the bottom. Click that, and the install runs.

Once, at the install rather than at every launch: what is started afterwards is
the Start-menu entry, which Windows put there itself and says nothing about. A
newer download is a different file and wants the same two clicks. The other way
round is to take the mark off before opening it instead: right-click the msi,
**Properties**, and tick **Unblock** at the bottom of the **General** tab.

**It installs into your own profile, and asks nobody for anything.** The app is
unsigned, so an installer asking for administrator would be an unsigned program
asking for the machine, and elevation buys a downloader nothing they wanted.
Everything therefore lands in the profile: the two exes under
`%LOCALAPPDATA%\Programs\Verkstead`, a **Verkstead** entry in your own Start
menu, and the uninstall entry in **Installed apps** beside everything else you
installed. A newer msi replaces the copy that is there rather than standing
beside it.

**And the install directory goes on your `PATH`**, which is the half of this
download that is not the icon at all: `verkstead ask`, `verkstead guide` and
the rest work in a terminal opened *after* the install. One that was already
open never read the entry — closing it and opening another is the whole of the
fix — and the uninstall takes the entry away with the files.

What is on the screen once **Verkstead** is opened from the Start menu is an
icon in the notification area, and the menu on it is the Linux tray's four:
**Open** brings the viewer back, and is what a double-click on the icon does;
**View Logs** opens the file under `%LOCALAPPDATA%\Verkstead` that the server's
logging goes to when there is no console to print it in; **Launch on Startup**
is a checkbox over a `net.tobico.Verkstead` value under
`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`; and **Exit** stops the
server. **Windows hides an icon it has not seen before**, in the flyout the `^`
on the taskbar opens — dragging it out of there onto the taskbar is what pins
it, and until you do, the app is running with its icon one click further away
than this describes.

**The shortcut opens the shim rather than the binary**, and that is what keeps
a console window off the screen: `verkstead.exe` is an ordinary console program
on every platform, because that is what a terminal, a session and a test all
want of it, so a shortcut naming it would put a black window in front of
whoever clicked. The shim is the windows-subsystem exe that starts the binary
beside it and exits with what it exited with, and there is nothing else about
the two files to keep track of.

**The startup registration is rewritten at every launch while it is there**,
with the path of what is running and the `desktop` verb behind it, so a
Verkstead installed somewhere else — a newer package, a profile that moved —
heals the entry the next time you start it by hand, and a machine that never
ticked the box is never registered. What the box cannot see is Windows' own
second opinion: the **Startup apps** tab in Task Manager, which Explorer
records separately, so switching Verkstead off there leaves the box ticked and
the value where it was.

`--no-open` starts it without the browser and `--data-dir` moves the Data
Directory off `%APPDATA%\Verkstead`, both of them for `verkstead desktop` run
from a terminal — an app started from the Start menu is started with no
arguments at all. Started that way there is no console to print in, so the log
file is the whole account of the run; started from a terminal, whatever stops
it is said there as well.

**Sessions run on Windows, and they run unsandboxed.** Two things stood between
a Windows Verkstead and a session — the pseudo-terminal and the Sandbox — and
this is the first of them alone: a session runs on a pseudoconsole Verkstead
opens for it, as an ordinary process of your own account's, with nothing between
it and the rest of the machine. Whatever you can reach, an agent working for you
can reach. The workbench says so rather than leaving it to be found out, in all
three of the places a session is set going, watched, or typed into: above
**Start work** on the composer, beside the terminal on the session pane, and on
a Conversation Terminal's own pane, that last one being a shell of yours with
the same reach. The AppContainer that closes it is a later stage's, and nothing
about this download changes when it lands.

**What a session does get is a profile of the Conversation's own**, under
`%APPDATA%\Verkstead\homes`, emptied and made again as each of that
Conversation's sessions starts. `USERPROFILE` and `HOME` point at it, and
`APPDATA`, `LOCALAPPDATA`, `TEMP` and `TMP` point inside it — so what npm
caches, what a tool writes down and what either of them throws away lands there
rather than in your own profile. The Agent Profile's account is joined into it,
every directory by a directory junction and every file by a hard link, so the
account an agent reads is the real one and a session starts logged in.

**A hard link wants one volume**, which is the one thing about this that can
refuse a session outright. Your account's directory and the Data Directory have
to be on the same drive; where they are not, the session does not start and the
log says which two paths those are and which of them to move. And a hard link
stops being one file the moment something saves over it by writing a temporary
file and renaming it into place, which is exactly how an agent saves its
config — so a linked file the session replaced is written back over the
account's own as the session ends, and the link is made fresh for the session
after. Nothing a session wrote to its account is lost.

**A Conversation Terminal opens on `pwsh`** where PowerShell 7 is installed and
on Windows PowerShell where nobody has installed one, in the Worktree, on the
same pseudoconsole a session runs on.

**The tools a session runs are the machine's**, as they are on a Mac: `git`,
`node`, `cargo` and whatever else an agent reaches for are found on the `PATH`
the server itself was started with, with Verkstead's own directory in front of
it — so an agent npm installed as a `claude.cmd` starts as readily as an
installer's `claude.exe`. Everything else there is what it is on the other two:
the Repos, the Briefs, the Question Sets, the Timeline, the pull requests.

Out of a checkout instead — the same server, told `--data-dir .` so that
`verkstead.db` and the rest land in the checkout rather than in the platform
directory — is [development.md](development.md#quickstart).

## A day's work

**Once per machine:** register the Repos you work in, and save at least one
**Agent Profile** — a claude home and config pair, and the models that account
can run. A Conversation fixes a **Grilling Pairing**, an **Implementation
Pairing** and a **Review Pairing** before it starts — a Profile and one of its
models, picked together as one row. The same Profile may fill all three, and
separate ones are how the parts bill to separate accounts. The review one runs
the wrap-up's review and nothing else, reviewing being a fresh set of eyes on
what was built — and its picker offers **No review** beside the accounts, for
work you would rather have wrapped up without one. The grilling picker offers
**No grilling** the same way, for work whose Brief is already the whole plan.
All of them are settled while the Conversation is drafting, and the work
starting is what fixes them.

**Then, per piece of work:**

1. **New conversation**, against a Repo. Write the **Brief** — the markdown
   document the work starts from, and its first Event. The base commit defaults
   to the default branch's tip and is yours to override.
2. **Start work.** The branch and the **Worktree** are made here, and a
   grilling session opens in the Sandbox. What it wants to know arrives as
   Question Sets on the Timeline and, if you have subscribed, on your phone.
   Answer from wherever you are; the session waits. On **No grilling** the same
   press skips to step 5 instead: one session builds from the Brief alone, and
   what the Brief leaves genuinely open comes back to you as a Question Set.
3. **The Proposal.** The grilling ends by proposing a **Direction** — inline,
   task list or roadmap — on a Set carrying the chooser. Picking one accepts
   the Proposal, and the pick is delivered back to the grilling session rather
   than acted on. Every other way of answering — a different Option, your own
   words, or leaving it open — sends it back, and the session decides for
   itself whether to keep grilling or propose again.
4. **The session produces what you picked.** A task list breaks the work into
   `.tasks/`; a roadmap stages it into `docs/roadmaps/`; inline writes the
   **Handoff** for the fresh session that builds it. That artifact, plus the
   session going quiet, is what ends the grilling and starts the pipeline —
   there is nothing left to press.
5. **It runs itself.** Each **Step** is one fresh session, ended when the file
   it turns on has gone from the Worktree *and* the commit removing it has
   landed *and* the session has gone quiet. Commits appear on the Timeline with
   their diffs. Where Verkstead cannot go on — a session that exited badly, or
   one that landed nothing — it **halts**: a **Notice** on the Timeline says
   what stopped and why, your phone is told, and nothing else is launched until
   you press **Resume**.
6. **The finish runs unattended.** The last Step pushes and opens a **draft
   pull request** per the target repo's `docs/agents/git-workflow.md`, and the
   Conversation moves to Wrapping. The PR is a pinned Event; its commits and
   comments are fetched through the host's `gh`.
7. **The wrap-up settles itself.** A fresh-context session reviews the PR and
   raises what it finds as a Question Set. Failing checks dispatch fix sessions
   — two failed attempts at the same check is where it stops and asks. New PR
   comments dispatch sessions that address them. The Conversation reaches
   **Done** when the checks are green, the review Set is answered, and nothing
   said on the PR is left unaddressed.
8. **Merging is yours.** Done means Verkstead has finished with the work, not
   that it is on `main`. Nothing in the pipeline merges anything.

**On a roadmap**, settling is also what starts the next **Stage**: a
Conversation of its own, on a branch stacked on the unmerged predecessor where
the repository's workflow records how, primed with the stage brief as its Brief
and Implementing from the first moment. A **Notice** on the Timeline says which
Stage started and where its branch went — or that the roadmap has no Stage left
to run. Nobody presses anything for either.

## What is different in practice

- **Questions belong to a Conversation.** There is no global queue to work
  through: a Set is on the Timeline of the work it came from, and it stays
  there, answered, afterwards. Nothing leaves a Timeline.
- **The checkout is not what gets worked in.** Every Conversation has its own
  Worktree under the Data Directory, so two pieces of work in one Repo no
  longer take turns, and the checkout you have open in an editor is not what a
  session is editing.
- **A run that stops is a thing on a page**, not a terminal you have to find.
  The stop Notice carries the evidence — which Step failed, how it ended, what
  git made of the Worktree, and the tail of what the session last said — read
  at the moment the run stopped and kept. Getting going again is one **Resume**,
  which works out what ought to be running now rather than replaying whatever
  failed; where you want the work to go somewhere else instead, **Steer** is
  what says so — pick the state to carry on in, write the instruction or the
  brief it needs, and the submit both moves the work and sets it going.
- **Review happens once, on the pull request.** This is what "no commit gates"
  buys: nothing pauses per commit, and everything you would have said there is
  said in the wrap-up instead.

## The old tools

`tobico-skills/roadrunner` and the `tobico-scripts` wrappers are left exactly
as they are: still on `PATH`, not deleted, and carrying no deprecation notice
in their own repositories. One person uses them and that person knows they are
retired, so a notice in a repository only they read would be ceremony rather
than warning.

Which also leaves them as the fallback while the switch-over is being made, and
that is the better reason not to touch them: they built this, up to and
including the stage that retires them, and a tool that still runs is worth more
than one removed the day its replacement first worked.
