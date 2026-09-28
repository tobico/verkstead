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

**Stacked branch** — one `gh stack view` names as a branch of a stack.
A roadmap stage with a chain under it is one even where `gh stack view`
errors, that being a worktree whose registry is empty rather than proof of
an unstacked branch: adopt the chain and join it first, the way
**Stacking roadmap stages** below says, and then come back here.

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

A roadmap's stages form one **chain**: each stage's branch is based on the
one below it, and each pull request is based on that branch. A stage is
cut from whatever was at the top of the chain when it started and
**joins** the chain at its finish — rebased onto whatever is at the top by
then, before its pull request opens, which is the only rebase it gets.
Only branch off `main`, unstacked, when the stage is genuinely independent
of any unmerged predecessor.

Prerequisite: `gh extension install github/gh-stack` (skip if
`gh stack --help` already works). Everything below was measured against
`gh stack` version 0.1.1 in a scratch repository rather than read off its
documentation; re-measure after an upgrade.

**The registry is per worktree.** `gh stack` keeps what it knows about a
stack inside the checkout it was run in, and every stage is worked in a
worktree of its own — so in a fresh one `gh stack view` exits 2 with
*current branch "…" is not part of a stack*, whatever GitHub holds.
Finding no stack is the normal case rather than a fault: the chain is
adopted again in each worktree that needs it.

**Adopting a chain of any depth takes one `init`**, bottom branch first
and the trunk left out — `init` infers the trunk:

    gh stack init <bottom-branch> <next> … <top-branch>

It adopts branches that already exist, so no history is rewritten by it.
A predecessor and one new branch is the same command rather than a
separate case:

    gh stack init <predecessor-branch> <new-branch>

Where this worktree has already adopted the chain, extend it instead:

    gh stack checkout <any-branch-in-the-stack>
    gh stack top
    gh stack add <new-branch>

Branch names are taken verbatim, so they keep the naming pattern. Leave a
newly added branch empty at creation time; the plan commit lands on it
normally (`gh stack add` only commits when passed `-m`/`-A`/`-u`).

Where nobody told you what the chain is, read it off GitHub rather than
off this checkout: `gh pr list` and each open PR's base, one pull
request's base being the branch of the one below it.

**Fetch, and hold every branch of the chain locally, before adopting
anything.** Adopting a branch this checkout has not got does not refuse:
it *creates* it, at the trunk's commit, and reports `✓ Adopted` as though
nothing were wrong. Everything that follows force-pushes whatever was
adopted, so a branch invented here is another stage's real work
overwritten on origin — measured: a five-branch `init` in a checkout
holding one of them, then a single `gh stack push`, moved four branches on
origin back to the trunk's commit. So first:

    git fetch origin
    git branch --track <branch> origin/<branch>   # for each one not here yet

**And read the adopted chain back before pushing anything.**
`gh stack view` prints it top-first and is safe when piped;
`gh stack view --json` is the form to parse, and the form to use if stdout
might be a PTY, where bare `view` opens a TUI. A chain that came back
wrong is one to stop at rather than to force-push.

**Joining: a branch that exists and is not at the top.** A stage cut
from a settled stage below the top is neither a first stacked stage nor
an extension of the chain — it is already a branch, with commits, and it
belongs above branches it was never based on. Adopt the chain with this
branch named last, and let the extension move it:

    gh stack init <bottom> … <top> <this-branch>
    gh stack rebase --no-trunk --upstack

`init` records where each branch actually sits rather than where the
argument order implies, so a branch not yet rebased onto the top comes
back from `gh stack view` with a `⚠` beside it and `needsRebase: true`
in `--json`. That marks this branch, and it does not single it out: a
trunk that has moved since the branches below were pushed sets it on
those too.

**Both flags, and neither is decoration.** A bare `gh stack rebase`
fetches the trunk and cascade-rebases the whole stack from the bottom
up, so the moment `main` has moved the branches below this one leave
their pushed commits behind — and the push that follows force-pushes
pull requests somebody is reading. Measured: three branches, one commit
on `main` after the lower two were pushed, and a bare `gh stack rebase`
moved all three off origin. `--upstack` rebases only from the current
branch up, `--no-trunk` leaves the trunk out of it, and with both the
branches below stay at origin's commit while this one still lands on top
of the one under it. Which is the whole of the join: this branch moves
and nothing else does.

A conflict exits 3 and stops in it, working tree and all. Resolve the
files, `git add` them, and `gh stack rebase --continue`;
`gh stack rebase --abort` restores the stack. Resolving is the job — a
conflict is two changes to reconcile, and taking one side wholesale
throws away work somebody did.

Then the **Finish sequence** above, whose `gh stack submit --auto` pushes
the chain and opens this branch's pull request.

### Updating a stack after review

Don't rebase stacked branches by hand — here or at the join, where
`gh stack rebase --no-trunk --upstack` is what moves the joining branch.
Here the whole stack moving is the point, which is why the flags the join
needs are not wanted: what follows is for branches that are already on
pull requests and are meant to be brought forward together. From any
branch in the stack, and in a worktree that has adopted the chain:

- `gh stack sync` — fetches, cascade-rebases each branch onto its updated
  parent, force-pushes atomically, and re-links the stack on GitHub. Use
  it after `main` moves or after an earlier PR in the stack merges. It
  never opens PRs.
- `gh stack rebase` — resolve conflicts interactively when `sync` reports
  one and backs out: exit 3, then `git add` and
  `gh stack rebase --continue`, or `gh stack rebase --abort`.
- Re-run `gh stack submit --auto` after adding a new branch to an
  existing stack.

### Exception: the release commits

`.github/workflows/release.yml` pushes two commits straight to `main` with no
pull request, and they are the only writes to `main` that bypass the process
above. Both are deliberate.

- **`chore: version <version>`**, before anything is built, touching
  `Cargo.toml`, `Cargo.lock` and `desktop/package.json`. The compiled binary
  reports the version it was compiled with, so the number has to be in the tree
  that gets built, and it has to be there before the build rather than after it.
  `v0.1.1` shipped a binary reporting `0.1.0` back when this was a human's job
  to remember: a fresh install offered to update itself to the version it
  already was. The third file is the same rule for the Windows installer, which
  electron-builder versions from that `package.json` and nothing else bumps —
  left alone, every Release's msi would read `0.1.0` in **Installed apps**.
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
