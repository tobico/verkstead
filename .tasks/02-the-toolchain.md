# 02. The toolchain moves to Rust 1.97, with wasm32-wasip2

## What to build

The human chose to take **wasmtime 50, which needs Rust 1.97** (Set 1772, Q5),
over staying on 48 under CI's 1.95.0 pin. They also chose for the dev shell to
get `wasm32-wasip2` from **a prebuilt pinned toolchain via rust-overlay**,
replacing nixpkgs' rustc (Q3). That decision was made when the pin was 1.95.0,
so the toolchain now pins 1.97.x on both sides. This task makes the move before
any wasmtime code depends on it:

- The flake's dev shell takes Rust from rust-overlay, pinned to the same exact
  1.97 release CI uses, with `wasm32-wasip2` and the components the shell has
  today (rustfmt, clippy, rust-analyzer, the source for `RUST_SRC_PATH`). The
  sccache wrapper and everything else in the shell keep working.
- Every `dtolnay/rust-toolchain@1.95.0` in `ci.yml` and `release.yml` moves to
  that 1.97 release. The legs that will build components add `wasm32-wasip2` to
  `targets:`.
- The workspace's `rust-version` moves up to match.
- Fix whatever new clippy lints and rustfmt changes 1.97 brings, so CI's Lint job
  is green.
- The `nix flake check` only runs on `main`, so build both checks locally,
  including the VM one. The memories on Nix checks and the clippy pin say how.
- `docs/development.md` says where the toolchain comes from now.

A scratch crate built for `wasm32-wasip2` in the new shell proves the target.
Nothing in the repository builds one until task 04.

## Acceptance criteria

- [ ] In the dev shell, `rustc --version` reports the pinned 1.97 release, and a
      scratch crate builds for `wasm32-wasip2`.
- [ ] CI and release pin the same release. The workspace builds, and clippy and
      fmt are clean under it.
- [ ] Both `nix flake check` checks build locally.
