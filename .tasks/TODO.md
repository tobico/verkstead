# Clonable repos

On the compose page, the Repo dropdown for whichever device the select names
lists that device's own Repos and then, behind a rule, a second group: the
repositories other members of the cluster have registered, by origin, that this
device lacks. Each row is the repository's name over where its origin is.
Picking one clones it on the named device into the parent **Create repo**
remembers there, over a path field where none is remembered yet, with that
device's own GitHub token. The clone is then registered and becomes the draft's
Repo, the same way a pick on an existing row lands.

The Transfer dialog's Preflight offers the same where it finds no match on the
device picked. It reads *clone it there*, and Go unlocks once the clone is done.
The work is cluster-wide rather than an Annex feature: the problem is the same
when starting a Conversation on any remote device, and an Annex only presets
where the clone lands (stage 03). The decisions are in ADR-0024, *Clonable
repos*, and the term is **Clonable Repo** in CONTEXT.md.

Roadmap stage: [02: Clonable repos](docs/roadmaps/wsl-annex/02-clonable-repos.md)

## Tasks

- [ ] 01: The clonable group in the dropdown — [details](01-the-clonable-group.md)
- [ ] 02: Clone from the dropdown — [details](02-clone-from-the-dropdown.md)
- [ ] 03: Clone from the Transfer dialog — [details](03-clone-from-the-transfer-dialog.md)
