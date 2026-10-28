# 01. Open the measuring pull request and record the baseline

## What to build

Every later task is measured by CI on one pull request, so this task opens it
and lays down the ledger the measurements go into. CI runs on pull requests
here and on nothing else, so without the pull request nothing after this is
measured.

Three changes ride on the first commit, each a lever in its own right:

- **Line tables only, for every build.** The workspace `Cargo.toml` gains a
  `[profile.dev]` with `debug = "line-tables-only"`. Panics keep file and line
  in their backtraces; a debugger shows no variable values in a dev build
  unless a developer overrides the profile in their own cargo config, which is
  the trade the human chose. The `test` profile inherits it.
- **The build step builds what the run uses.** The Windows job's build step
  builds `--workspace --tests --bins` rather than `--all-targets`: the two
  programs under the server crate's `examples/` are developer probes that no
  test runs, and the sessions suite wants only the `verkstead` binary beside
  its own.
- **The ledger.** The Windows job's long comment in `.github/workflows/ci.yml`
  is rewritten around a table: the budget (every leg under ten minutes, the
  typical run near seven), the baseline (the figures in `TODO.md`, from the
  last green run on `main` before this work), and a row per lever to come —
  the merge in two steps, Defender, the legs, rust-lld, the template
  repositories — each to be filled with the build and test time of both legs
  on the run that measured it. The comment keeps what it says today about
  *why* the job exists and what the suites prove; what goes is the stale
  reasoning about two legs, which task 05 replaces.

Then push the branch, open a **draft** pull request against `main` for it if
none exists, wait for its run, and write the first row of the ledger: both
legs' build and test times after these three changes, read off the run's job
timings. The figures go in a second commit, so that the ledger in the tree
describes a run that has happened.

## Acceptance criteria

- [ ] `[profile.dev]` in the workspace `Cargo.toml` sets `debug = "line-tables-only"`, with a comment saying why
- [ ] The Windows job's build step no longer builds the examples, and the sessions suite still finds `verkstead.exe` beside its binary
- [ ] A draft pull request for this branch is open against `main`, and the ledger in the Windows job's comment holds the baseline and this commit's leg times
