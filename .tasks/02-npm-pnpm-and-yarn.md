# 02. npm, pnpm and yarn

## What to build

The `node` **Descriptor**, carrying the three tools that talk to the npm
registry. One entry, one box on the **Language support** pane: switching Node
off switches all of them off, which is what was decided.

Each tool's own variable, checked against that tool's current documentation as
this task starts. Two of the three are worth naming as traps rather than left to
be discovered:

- **pnpm's store is not `PNPM_HOME`.** That is where pnpm puts global
  binaries. The content-addressable store is its own setting, and the adoption
  doc's worked example gets this wrong today — see task 07.
- **There are two yarns**, and this entry carries both: Classic reads one
  variable, Berry reads another and defaults to a cache inside the project
  rather than a shared one at all.

**pnpm is the first built-in to name `{stores}`** — the directory beside the
Worktrees — because pnpm hardlinks packages out of its store into the project
and falls back to copying the lot where the two are on different filesystems,
which the Build Cache is free to be. A copy out of the store still fetches
nothing, so none of the install proofs can see that fall-back; what can see it
is the link count on a file in the project, or the tool's own words.

The proof per tool is the harness task 01 built: two installs at once in two
Sandboxes, then a third denied its registry, skipped where the tool is missing
and failing the build in CI where it is skipped. npm, pnpm and yarn each
document an offline flag; use it rather than pointing a source nowhere.

## Acceptance criteria

- [ ] The `node` entry sets npm's, pnpm's and both yarns' store variables for
      every session on all three platforms
- [ ] Per tool: two installs at once, then a third denied its registry, which
      succeeds
- [ ] pnpm's store is under `{stores}`, and a file in the Worktree's installed
      packages has a link count above one
- [ ] A Repo pinning its own store location in its own config still wins over
      the variable the session was given
