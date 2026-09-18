# First-run hygiene

A first run on a fresh Windows machine fails only where it has to, and says so
by name. A `claude` that is the desktop app rather than the CLI is recognised as
that and refused with the npm install named, on the wizard's row and at session
start. The desktop's log holds no workbench key and opens with a byte-order mark,
so the tray can go on opening it. A tag whose manifest disagrees with it fails
before anything is built. The token field says which scopes a token needs. And
two things that read as broken at every start — the `containers\standing`
warning and the mixed-separator prompt path — are put right.

Every one of them is a finding from an independent Windows install of `v0.1.1`.
The stage depends on no other, and it carries the release gate that matters
before `v0.1.2` is tagged.

Roadmap stage: [05: First-run hygiene](docs/roadmaps/built-roots/05-first-run-hygiene.md)

## Tasks

- [x] 01: The desktop app refused — [details](01-the-desktop-app-refused.md)
- [ ] 02: The desktop's log file — [details](02-the-desktops-log-file.md)
- [ ] 03: The release gate — [details](03-the-release-gate.md)
- [ ] 04: The token note and the docs — [details](04-the-token-note-and-the-docs.md)
- [ ] 05: The sweep's warning and the prompt path — [details](05-the-sweeps-warning-and-the-prompt-path.md)
