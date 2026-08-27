# Visibility

Work in companion repos is visible where the main repo's already is. Commits on
a read-write companion's branch land on the Timeline as they happen, labeled
with the repo they belong to, and the diff under a Question Set shows every
repo's uncommitted changes — the main repo's included, now composed by the
server rather than by the CLI.

Demonstrable end to end: with a read-write companion configured, a session that
commits in it has that commit on the Timeline within a sweep, labeled and
readable; and a Set asked while both worktrees are dirty carries both repos'
uncommitted changes as labeled blocks, with the table of contents naming the
files under each repo.

Roadmap stage: [02: Visibility](docs/roadmaps/companion-repos/02-visibility.md)

## Tasks

- [x] 01: Per-companion commit sweeps — [details](01-per-companion-commit-sweeps.md)
- [x] 02: The server composes the Diff — [details](02-the-server-composes-the-diff.md)
- [ ] 03: A block per repo under the Set — [details](03-a-block-per-repo.md)
