# 03. deno and bun

## What to build

deno's and bun's stores, added to the **`node` entry task 02 created** rather
than to entries of their own: the five JavaScript tools are one Descriptor and
one box on the **Language support** pane.

Each reads one variable of its own, checked against its current documentation as
this task starts. Two things differ from task 02's three:

- **bun hardlinks out of its store into the project**, as pnpm does, so bun's
  goes under `{stores}` — the directory beside the Worktrees — and the same
  link-count evidence is what says it really hardlinked rather than silently
  copying. deno's can go under `{cache}` unless this task finds otherwise.
- **bun documents no offline flag.** Where a tool has one, the third install
  uses it — deno's is `--cached-only`. Where it has none, the third install is
  given a **registry pointed somewhere unreachable** instead, which is the same
  proof by the other route.

Everything else is the harness task 01 built: two installs at once in two
Sandboxes against one store, then a third denied its registry; skipped with a
line naming the tool where it is missing, and that skip failing the build in CI.
Both tools go in the dev shell and in the CI job, neither being on the runner
image.

## Acceptance criteria

- [ ] The `node` entry also sets deno's and bun's store variables, for every
      session on all three platforms
- [ ] Per tool: two installs at once, then a third denied its registry —
      deno's with `--cached-only`, bun's against a registry pointed nowhere
- [ ] bun's store is under `{stores}`, and a file in the Worktree's installed
      packages has a link count above one
- [ ] Both are in the dev shell and in the CI job, and a missing one is skipped
      locally and red in CI
