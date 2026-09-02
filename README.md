# Verkstead

A management platform for agentic coding. Everything is driven from a web GUI:
a background orchestrator creates worktrees, runs and monitors sandboxed coding
sessions, puts question sets and commits to you, and works through task lists
and staged roadmaps unattended.

Verkstead (Norse *verk*, work + *stead*: a workshop) began as a clone of
[askance](https://github.com/tobico/askance) and keeps its architecture — a
Rust workspace and a SolidJS SPA in one binary, SQLite, SSE nudges, web push,
and server-side rendering of everything an agent writes. askance remains a
separate, maintained product; Verkstead diverges freely from it.

## Status

**Early, private, and unreleased.** There is nothing to download yet. The
workbench, the sandboxed sessions, the task-list and roadmap pipelines and the
per-PR wrap-up are built; what has not happened yet is a real repository driven
through them end to end, which is where [adoption](docs/adoption.md) stands.

What a tag will produce is four ways in rather than one, and no two of them
are the same thing. On a host that is always on, the flake builds the headless
daemon and the NixOS module runs it. On a Linux desktop,
`Verkstead-x86_64.AppImage` is that same server started from an icon: the viewer
in your browser and a tray icon over it — or no icon at all where the desktop
has no tray host, vanilla GNOME being the case people meet, and it serves just
the same. On a Mac, `Verkstead-universal.dmg` carries `Verkstead.app` — one
download for both Macs, the same server again, with its icon in the menu bar.
That app is unsigned, so the first launch is refused and System Settings is
where it is allowed through; the steps are written out beside the download in
[adoption](docs/adoption.md#the-desktop-app-on-a-mac). On Windows,
`Verkstead-x86_64.exe` is the whole download: no installer, and it runs from
wherever you put it, with its icon in the notification area. It is unsigned
there too, so SmartScreen stops the first launch behind a **More info** link
with **Run anyway** under it — also written out beside the download in
[adoption](docs/adoption.md#the-desktop-app-on-windows). Which of the four you
want is [adoption](docs/adoption.md#getting-it-running).

**Sessions run on Linux and on a Mac**, over the mechanism each platform has
and to one description of what a session may reach: bubblewrap, where the rest
of the machine is not in the session's namespace at all, and Apple's sandbox,
where the machine is in plain sight and refused. What is inside is the same
either way, and [adoption](docs/adoption.md#the-desktop-app-on-a-mac) says what
a Mac session can and cannot get to. **Windows has everything but those**: an
agent works in a terminal and Windows has none to give it yet, so the workbench
says so where a session would be started rather than failing to start one, and
a later stage is where they arrive. The daemon install is the NixOS module's.

## Where things are written down

**[Design](docs/design/verkstead.md)** — what Verkstead is and the decisions it
rests on, as settled in the planning session behind it.

**[MVP roadmap](docs/roadmaps/mvp/ROADMAP.md)** — the five stages from here to
a Verkstead that covers the whole loop, and the brief for each.

**[CONTEXT.md](CONTEXT.md)** — the project's vocabulary. Conversation, Brief,
Timeline, Question Set, Answer and the rest, defined once.

**[Adoption](docs/adoption.md)** — what Verkstead replaces, how to get it
running, and how a day's work goes through it from Brief to settled pull
request.

**[Development](docs/development.md)** — the dev shell, building the viewer,
and the loop for working on Verkstead itself.

**[Releasing](docs/releasing.md)** — how a tag would become the published
binaries, the AppImage, the dmg and the exe. Nothing has been released under
this name yet.

## License

MIT — see [LICENSE](LICENSE).
