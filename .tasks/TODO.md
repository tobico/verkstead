# The instructions text

One text on the settings page reaches every session, whatever harness runs it.
It is written into the Built Root as the file that harness reads as its global
instructions, so what a human wants every session to know is said once and is
there before a session asks anything. The human's own global `CLAUDE.md` does
not travel — a Built Root holds none of the account's rules — and this is what
takes its place.

One text for every Profile, kept in `config.yaml` beside the other things
Verkstead is told rather than finds, and read at session start like the rest of
it: a change on the settings page applies to the next session and a running one
keeps what it started with. The Repo's own `CLAUDE.md` or `AGENTS.md` in the
Worktree is untouched, and the settings text sits above it the way the human's
global file used to.

Roadmap stage: [03: The instructions text](docs/roadmaps/built-roots/03-instructions-text.md)

## Tasks

- [x] 01: The setting — [details](01-the-setting.md)
- [ ] 02: The file in every root — [details](02-the-file-in-every-root.md)
- [ ] 03: The docs — [details](03-the-docs.md)
