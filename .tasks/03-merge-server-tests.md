# 03. One test binary for the server, and the seven that stay alone

## What to build

The same move for the server crate, whose 76 test files are most of the build:
all but seven move under `tests/suite/` and are declared by its `main.rs`, as
task 02 did for the other three crates. The seven stay where they are, each a
test binary of its own, because their safety argument is being the only test in
their process: `per_user_path`, `servers_path`, `session_path` and
`vendor_installers` move `HOME` and `PATH` on the process and look again;
`advertising_opt_out` and `update_opt_out` set and clear an opt-out variable;
and `withdrawing` raises `SIGTERM` at the process it is in. Each says so at its
head, and that argument is what keeps them out of the merge. The `site`
fixture directory is reached through `$CARGO_MANIFEST_DIR` and does not move.
The `sessions_windows` suite finds the built `verkstead.exe` two directories
above the running test binary, which holds for a binary named `suite` as it
did before.

Two configurations name the server's test binaries and change with them:

- **The nextest serial group** in `.config/nextest.toml` holds the suites that
  start processes as the session account one at a time across all of them,
  and the one terminal test that wants the machine to itself, by `binary_id`.
  Those become filters on the `verkstead-server::suite` binary by test-name
  prefix — `sessions_windows::`, `logins_windows::`, `sandbox_windows::`,
  `account_windows::` — beside the cli's two, which task 02 already moved onto
  `verkstead-cli::suite`, and the in-crate `sandbox::granting::writing` tests as
  before. Prove the group still holds every test it held by listing the filter
  with nextest before and after and comparing.
- **The dev docs.** `docs/development.md` says how one suite is run by hand now
  that a suite is a module: `cargo test -p verkstead-server --test suite --
  conversations::` runs the conversations suite and nothing else. Say it where
  the document first runs the tests, and anywhere it names a test binary.

Then the Linux job's threads: the server's suites stand real servers up and
run real processes, and some took a port, a static or a temp directory for
granted as one process's own. Read each suite's statics and fixtures for what
is now shared across the merged binary under `cargo test`, and fix what races.
The `VERKSTEAD_TEST_PACE` factor the Linux job sets is for the sessions suite's
clocks and is not the tool for a race.

When everything is green, record the build time of each leg in the ledger.
This is the row that should show the largest fall.

## Acceptance criteria

- [ ] The server crate builds eight test targets — `suite` and the seven named above — and `cargo test --workspace` on Linux and the Windows legs on nextest are green
- [ ] `cargo nextest list` with the serial group's filter names the same tests it named before the merge, and the machine-alone terminal test is still filtered to itself
- [ ] `docs/development.md` says how one suite is run by hand, and the ledger carries both legs' build times from the run that measured this commit
