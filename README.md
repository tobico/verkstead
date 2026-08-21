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

**Early, private, and unreleased.** There is no binary to download — the flake
builds it and the NixOS module runs it. The workbench, the sandboxed sessions,
the task-list and roadmap pipelines and the per-PR wrap-up are built; what has
not happened yet is a real repository driven through them end to end, which is
where [adoption](docs/adoption.md) stands.

NixOS only for now: Linux and bwrap are a hard requirement.

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
binaries. Nothing has been released under this name yet.

## License

MIT — see [LICENSE](LICENSE).
