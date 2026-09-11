# 03. Run the Windows suite through cargo-nextest

## What to build

The Windows job's test step runs the workspace's ~45 test binaries one after
another — cargo runs each executable serially, so the wall-clock is the sum of
every suite. cargo-nextest runs tests from all binaries concurrently, which is
the largest lever left once task 01 removes the boundary cost.

Install a pinned `cargo-nextest` in the `Windows` job (a released version, as
everything else in CI is pinned), and run the suite with `cargo nextest run`
where it used `cargo test`. Keep the same feature flags the Windows job already
passes (`--features verkstead-desktop/shim`) and the same environment.

Add a nextest configuration (`.config/nextest.toml`) that holds serial the
tests which share machine-global state, so concurrency doesn't make them flake:

- The Launch-on-Startup **Run-key** tests, which read and write one shared
  value under `HKEY_CURRENT_USER` — two running at once would race on it.
- The suites that start real sessions as the shared session account. Windows'
  secondary-logon service performs one logon at a time machine-wide; heavy
  concurrent logons back up against that. Put these in a test-group with a
  bounded concurrency (or serial) so the retry window isn't overrun.

Account for doc-tests: `cargo nextest run` does not run them. All doc-tests in
the workspace are currently empty, so nothing is lost — but make that explicit
(either accept it with a note, or run `cargo test --doc` as a cheap separate
step) so a future doc-test isn't silently skipped.

Scope this to the `Windows` job only — the Linux and macOS jobs keep plain
`cargo test` (settled during grilling).

## Acceptance criteria

- [ ] The `Windows` job installs a pinned cargo-nextest and runs the suite with `cargo nextest run`, carrying the same features and environment as before
- [ ] A `.config/nextest.toml` holds the Run-key tests and the session-account/logon suites serial (or in a bounded group), and they stay green under concurrency
- [ ] Doc-tests are explicitly accounted for rather than silently dropped
- [ ] The whole workspace passes on Windows through nextest
- [ ] The Linux and macOS jobs are unchanged
