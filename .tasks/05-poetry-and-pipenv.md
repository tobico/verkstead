# 05. poetry and pipenv

## What to build

poetry's and pipenv's caches, added to the **`python` entry task 04 created**,
and with them the one question those two tools raise that pip and uv do not.

**Both keep their virtual environments under the directory their cache variable
names.** Sharing that directory between Conversations would therefore share
venvs, and a venv holds absolute paths — one built in another Worktree is broken
in this one, and two sessions racing on a shared one is worse than a cold
install. So each tool is **told to put its virtual environment in the project**,
which is its own documented setting and needs no new placeholder: a descriptor's
value does not have to be a path. Downloads are shared; the environment stays in
the Worktree.

Each tool's cache variable and each tool's in-project setting are checked against
that tool's current documentation as this task starts. Both drive pip or uv
underneath, so the variables task 04 set are doing work here too — worth
asserting rather than assuming, since a tool that ignores its own variable and
falls through to pip's would look like a pass.

The proof per tool is the harness task 01 built: two installs at once in two
Sandboxes against one store, then a third denied its registry. Neither tool is
on the runner image, so both go in the dev shell and the CI job.

## Acceptance criteria

- [ ] The `python` entry also sets poetry's and pipenv's cache variables and
      tells each to keep its virtual environment in the project
- [ ] Per tool: two installs at once, then a third denied its registry, which
      succeeds
- [ ] The virtual environment is in the Worktree and nothing of it is in the
      shared store, asserted by looking in both
- [ ] Both are in the dev shell and in the CI job, and a missing one is skipped
      locally and red in CI
