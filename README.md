# Verkstead

A management platform for agentic coding. Everything is driven from a web GUI:
a background orchestrator creates Worktrees, runs and monitors sandboxed coding
sessions, puts Question Sets and commits to you, and works through task lists
and staged roadmaps unattended.

Verkstead (Norse *verk*, work + *stead*: a workshop) began as a clone of
[askance](https://github.com/tobico/askance) and keeps its architecture — a
Rust workspace and a SolidJS SPA in one binary, SQLite, SSE nudges, web push,
and server-side rendering of everything an agent writes. askance remains a
separate, maintained product; Verkstead diverges freely from it.

## Status

**Released, and early.** Each version is published on
[GitHub Releases](https://github.com/tobico/verkstead/releases), and the latest
is what any of the four ways in below fetches, and what the website,
<https://tobico.github.io/verkstead/>, links to. It is a single-user workbench,
and still young enough to change a good deal from one release to the next.

There are four ways in, and they are four different things rather than four
spellings of one. Which you want is which machine you are describing; two at
once is two Verksteads, and the second to reach port 8422 says so and exits.

**The NixOS module**, for a host that is always on. The flake builds the
headless daemon and `services.verkstead` runs it, with `paths` naming the
repositories and agent accounts it may reach — a directory not named there is
not there as far as the service is concerned — and `sandboxBinds` the
Sandbox Configuration.

**`Verkstead-x86_64.AppImage`**, on a Linux desktop: that same server started
from an icon, the workbench in a window of its own and a tray icon beside it —
or no icon at all where the desktop has no tray host, and the window is there
just the same. Make it executable and run it; sessions want the distribution's
`bubblewrap` package installed, since it cannot ride inside an AppImage.

**`Verkstead-universal.dmg`**, on a Mac: one download for Apple silicon and
Intel alike, the workbench in a window, a tile in the Dock and an icon in the
menu bar. Drag **Verkstead** into Applications and open it. The app is signed
and notarized by Apple ([ADR-0027](docs/adr/0027-cluster-updates.md)), so the
first launch asks only whether to open an app downloaded from the Internet:
click **Open**, and it does not ask again.

**`Verkstead-x86_64.exe`**, on Windows: an installer that puts the same app
into your own profile, at `%LOCALAPPDATA%\Programs\Verkstead`, with no
administrator asked for, an icon in the notification area and `verkstead` on
your `PATH` for a terminal opened after the install. It is unsigned, so
SmartScreen stops it behind *Windows protected your PC*: click **More info**,
then **Run anyway**. Once per download; right-clicking the exe, **Properties**,
and ticking **Unblock** does the same beforehand. It stays unsigned because
every route open to its maintainer, an individual in Australia, was weighed and
declined, and a signed build would start behind the same window until downloads
earned it a reputation
([ADR-0020](docs/adr/0020-electron-desktop.md#amended-the-windows-build-stays-unsigned)).

**A machine that has the earlier Windows Installer package of Verkstead**
installs this exe over it once, by hand. The installer finds the old install
and removes it before putting its own in the same place, and the Data
Directory, the settings and Launch on startup are left as they were.

**Taking it off Windows again** starts before Verkstead is uninstalled, because
the uninstaller knows nothing of the account sessions ran as or of what was
granted to it. Quit Verkstead, then run `verkstead session-account remove` in
an elevated terminal. It reads every record Verkstead kept under its Data Directory of the
access-control entries it wrote on your directories, takes each one off with a
line saying which, and then deletes the account, the `C:\Users` directory
Windows made for it the first time a session ran, and its password out of the
secrets file. A machine that has run Verkstead and then had it taken off should
look as it did, and the `entries = 6 of 6 taken off` line it prints is how you
check. A grant that will not come off is named, and then nothing else is
deleted, so the path can be put right and the command run again; `--force`
deletes the account regardless, still naming the paths. Run again on a machine
whose account is already gone, it takes off whatever entries the records still
name. A Conversation's own grants, on its Worktree and the Repo's git
directory, stand for as long as the Conversation is open, Verkstead running or
not, and come off when it closes or when the account is removed. Then
uninstall Verkstead from **Installed apps**.

**Sessions run on all three, and on all three behind a boundary.** One
description of what a session may reach — its Worktree, the Repo's git
directory, the Sandbox Configuration's entries, the Build Cache, a home of its
own with a Built Root of the Agent Profile's account inside it, the network,
and nothing else of the machine — rendered over the mechanism each platform
has. On Linux it is bubblewrap, and the rest of the machine is not in the
session's namespace at all. On a Mac it is Apple's sandbox, `sandbox-exec`,
and the machine is in plain sight and refused. On Windows it is a local account
of Verkstead's own, with an access-control entry written for it on each
directory the description names; making that account is the one elevated step
Verkstead asks for, and the setup wizard takes it behind a User Account Control
prompt, or `verkstead session-account create` in an elevated terminal does.
A boundary that cannot be made refuses the session rather than running it
without one.

## Where things are written down

**[Design](docs/design/verkstead.md)** — what Verkstead is and the decisions it
rests on, as settled in the planning session behind it.

**[MVP roadmap](docs/roadmaps/mvp/ROADMAP.md)** — the five stages from here to
a Verkstead that covers the whole loop, and the brief for each.

**[CONTEXT.md](CONTEXT.md)** — the project's vocabulary. Conversation, Brief,
Timeline, Question Set, Answer and the rest, defined once.

**[Copy](docs/copy.md)** — how Verkstead speaks to the human: the rules for
every string on a screen, and the reasons behind them.

**[Development](docs/development.md)** — the dev shell, building the viewer,
and the loop for working on Verkstead itself.

**[Releasing](docs/releasing.md)** — the one workflow that turns a version
number into the published binaries, the AppImage, the dmg and the Windows
installer.

## Licence

MIT — see [LICENSE](LICENSE).
