# 02. One test binary each for store, schema and cli

## What to build

The test files of the store, schema and cli crates become one test binary per
crate, named `suite`. The files move under each crate's `tests/suite/`, and a
`main.rs` there declares each as a module; Cargo discovers `tests/suite/main.rs`
as a test target on its own, so the manifests need no `[[test]]` entries and
`autotests` stays on. Nothing in a test changes but its position: a file's
inner `#![cfg(...)]` attribute still gates it as a module, its `//!` docs are
the module's, and names that clashed between files are now in modules of their
own.

Two things do change. Every cli test file declares `mod support;` today, and
two store files declare `mod owning;`; in the merged binary the shared module
is declared once in `main.rs` and the files reach it as `super::support` and
`super::owning`. And the cli tests run the built `verkstead` binary through
`CARGO_BIN_EXE_verkstead`, which is the manifest's and unaffected by the name
of the test binary; check that nothing in these three crates assumed the old
binary names anywhere else.

Under nextest on Windows nothing is shared between tests, every test being a
process. Under `cargo test` on Linux the tests of one binary run on threads of
one process, which is where a merge could race: none of these three crates'
tests write to the process environment or signal their own process — the seven
that do are the server's, and task 03 keeps them alone — but a static that was
once per binary is now once per crate's suite, and a port, a temp directory or
a lock taken for granted has to be read for it.

When the Linux job and both Windows legs are green, record the build time of
each leg in the ledger.

## Acceptance criteria

- [ ] The store, schema and cli crates each build exactly one test target, named `suite`, and `cargo test --workspace` on Linux and the Windows legs on nextest are green
- [ ] `support` and `owning` are declared once each in their crate's `main.rs`
- [ ] The ledger carries both legs' build times from the run that measured this commit
