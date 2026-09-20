# Git workflow

## Branch naming

Pattern: `<feature>`

## Review process

### Finish sequence

**Unstacked branch** — the normal case:

1. Push the current feature branch to origin.
2. Open a **draft** PR (`gh pr create --draft`). Title = feature name;
   body = summary of the completed tasks (name the stage and roadmap if
   this was a roadmap stage). Return the PR URL.

**Stacked branch** — created with `gh stack init`/`gh stack add`:

1. `gh stack submit --auto` — pushes every branch in the stack, opens a
   PR for each branch that lacks one, repoints the base of existing PRs,
   and creates or updates the Stack on GitHub. **Always pass `--auto`**:
   bare `gh stack submit` opens an interactive editor an agent can't
   drive. With `--auto` new PRs are created as drafts (`--open` would
   make them ready for review — don't).
2. `--auto` auto-generates PR titles, so correct this branch's PR:

       gh pr edit <branch> --title '<feature name>' --body '<summary>'

   Leave the stack's other PRs alone — they belong to finished stages.
3. `gh stack view` lists the stack's PRs. Return this branch's PR URL.

### Stacking roadmap stages

When a new roadmap stage's predecessor is finished but its PR is **not
yet merged**, and the new stage builds directly on that work, put the new
stage on a stacked branch. Only branch off `main`, unstacked, when the
stage is genuinely independent of any unmerged predecessor.

Prerequisite: `gh extension install github/gh-stack` (skip if
`gh stack --help` already works).

- **First stacked stage** — adopt the predecessor as the stack bottom:

      gh stack init <predecessor-branch> <new-branch>

  `init` adopts branches that already exist and creates the ones that
  don't, so the predecessor's history is left untouched.

- **Stack already exists** — extend it:

      gh stack checkout <any-branch-in-the-stack>
      gh stack top
      gh stack add <new-branch>

Both keep the branch-naming pattern. Leave the branch empty at creation
time; the plan commit lands on it normally (`gh stack add` only commits
when passed `-m`/`-A`/`-u`).

### Updating a stack after review

Don't rebase stacked branches by hand. From any branch in the stack:

- `gh stack sync` — fetches, cascade-rebases each branch onto its updated
  parent, force-pushes atomically, and re-links the stack on GitHub. Use
  it after `main` moves or after an earlier PR in the stack merges. It
  never opens PRs.
- `gh stack rebase` — resolve conflicts interactively when `sync` reports
  one and backs out.
- Re-run `gh stack submit --auto` after adding a new branch to an
  existing stack.

### Exception: the release commits

`.github/workflows/release.yml` pushes two commits straight to `main` with no
pull request, and they are the only writes to `main` that bypass the process
above. Both are deliberate.

- **`chore: version <version>`**, before anything is built, touching
  `Cargo.toml` and `Cargo.lock`. The compiled binary reports the version it
  was compiled with, so the number has to be in the tree that gets built, and
  it has to be there before the build rather than after it. `v0.1.1` shipped a
  binary reporting `0.1.0` back when this was a human's job to remember: a
  fresh install offered to update itself to the version it already was.
- **`chore: release manifest for <tag>`**, after the Release, touching
  `nix/release.json` and nothing else. That manifest is the entire interface
  to the binary flake: a version, plus a url and an SRI hash per nix system,
  which the flake fetches by and verifies against with nothing hand-edited in
  between. Upkeep has to be zero per release. A manifest that needed a merge
  would go stale the first time somebody was busy, and a stale one is a broken
  install for everyone downstream — which is exactly the audience least able
  to diagnose it.

What follows from the exception:

- Both commits are authored by `github-actions[bot]`, so `main`'s history
  carries commits belonging to no person. That is the intent: nobody wrote
  them, and attributing them to whoever started the run would be a fiction.
- A push made with the workflow's own `GITHUB_TOKEN` starts no workflow run,
  so CI never sees either commit. That is what stops the pushes from looping.
- For the manifest, it is safe because the commit touches one generated file
  that no check reads. **The version bump is the other case**: it touches
  files the checks do read, which is why the release runs `ci.yml` itself as
  its first job, on the tree the bump is about to land on. What reaches `main`
  unreviewed is a version number on a tree that was green a job earlier.
  Anything landing this way that was neither generated nor checked would be
  unreviewed *and* unchecked.
- Branch protection on `main` requiring a pull request would reject both
  pushes and fail the release run. If protection is ever turned on, the
  workflow's identity needs a bypass, or these jobs need another route.

### Notes

- Default branch: `main`, on `origin` (github.com/tobico/verkstead, **private**
  — and staying so until Verkstead works, per
  [the design](../design/verkstead.md#product-decisions)).
- PRs open as draft. Change "draft" to "ready" here to open ready-for-review.
- **The history before this repo existed is askance's.** Verkstead is a clone
  of github.com/tobico/askance taken at `6f32b11`, so everything up to that
  commit was written under the other name and against the other origin. Two
  things in it are worth knowing when reading back: this pull-request process
  replaced a direct-merge one on 2026-08-08, so commits up to `e35ea47` landed
  on `main` with no PR to find; and the public/private flip recorded around
  that date was askance's, not this repo's.
- HTTPS over SSH: SSH to github.com fails on this machine with `Bad owner or
  permissions on …/ssh_config.d/20-systemd-ssh-proxy.conf`, so `origin` is an
  HTTPS URL authenticated by `gh`/`GITHUB_TOKEN`. That token needs the
  `workflow` scope to push anything under `.github/workflows/`.
