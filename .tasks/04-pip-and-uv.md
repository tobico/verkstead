# 04. pip and uv

## What to build

The `python` **Descriptor**, carrying pip's download cache and uv's. One entry
and one box on the **Language support** pane for the whole of Python; poetry and
pipenv join the same entry in task 05.

Each tool's own variable, checked against its current documentation as this task
starts. The one thing to settle beyond that:

- **uv hardlinks out of its cache into the environment** and falls back to
  copying where the cache and the project are on different filesystems, so uv's
  goes under `{stores}` — the directory beside the Worktrees. uv says so in its
  own output when it could not hardlink, which is the second piece of evidence
  worth asserting beside the link count. pip copies, so pip's goes under
  `{cache}`.

The proof per tool is the harness task 01 built: two installs at once in two
Sandboxes against one store, then a third **denied its registry** — `--no-index`
for pip, `--offline` for uv — skipped with a line naming the tool where it is
missing, and that skip failing the build in CI. The runner image carries a
python3 with pip; uv it does not, so uv goes in the dev shell and in the CI job.

Only downloads are shared. A virtual environment holds absolute paths, so one
built in another Worktree is broken in this one — nothing of a venv belongs in a
shared store. pip and uv both leave the venv where the session put it, so there
is nothing to configure here — task 05 is where the two tools that do not leave
it alone are dealt with.

## Acceptance criteria

- [ ] The `python` entry sets pip's and uv's cache variables for every session
      on all three platforms
- [ ] Per tool: two installs at once, then a third denied its registry —
      `--no-index` for pip, `--offline` for uv — which succeeds
- [ ] uv's cache is under `{stores}`, and both the link count on an installed
      file and uv's own words on the install say it hardlinked
- [ ] uv is in the dev shell and in the CI job, and a missing tool is skipped
      locally and red in CI
