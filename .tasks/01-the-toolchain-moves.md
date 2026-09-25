# 01. The toolchain moves to nixos-26.05

## What to build

The flake's nixpkgs moves from `nixos-25.11` to `nixos-26.05`, and everything
pinned to match moves with it. Nothing of Electron arrives in this task: what it
delivers is the same workspace, the same viewer and the same checks, green on the
newer toolchain, so that the task after it can put an Electron in the dev shell
without a toolchain argument underneath it.

**The bump is for Electron and nothing else.** The app is developed against the
dev shell's Electron and packed against the one `package.json` names, and the two
being different majors would be a dev-only Electron that is not what ships
(ADR-0020 leaves electron-builder's downloads to CI). `nixos-25.11` carries 41,
which is already outside the newest three majors Electron supports;
`nixos-26.05` carries 43, which is inside them. No release branch carries 44, and
an unstable channel was rejected rather than have the dev shell move under the
project whenever the lock is refreshed.

**What comes with it, measured rather than guessed:** rustc and clippy 1.91.1 →
1.95.0, Node 22.22.2 → 24.21.0, pnpm 10.28.0 → 11.27.0. Each of those three is
pinned a second time by hand outside the flake — `ci.yml` installs the Rust
toolchain by version branch, and `.github/actions/setup-viewer` names the Node
and pnpm versions the release workflow shares — and each of those pins carries a
comment saying it is the flake's. So the pins move together or the comments start
lying.

**Where the fallout will be.** Clippy runs over the whole workspace with
`-D warnings`, so four minors of new lints land on code this stage never touched;
they are fixed here rather than left for a later task to trip over. A pnpm major
may want the lockfile rewritten, and `web/pnpm-lock.yaml` is `lockfileVersion:
9.0` today — a lockfile that changes shape is part of this task, not a surprise
in the next one. Node 24 is what the viewer's suite then runs under.

**And `nix flake check` is nobody's CI on a branch.** It runs on pushes to `main`
alone, and it is what builds the viewer hermetically and boots the NixOS module in
a VM — both of which read the nixpkgs this task moves. So it is built locally,
VM included, rather than discovered after the merge.

## Acceptance criteria

- [ ] `flake.nix` is on `nixos-26.05`, `flake.lock` is refreshed, and a fresh
      `nix develop` gives rustc 1.95.0, Node 24.21.0 and pnpm 11.27.0.
- [ ] `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`
      and `cargo test --workspace` are green on the new toolchain, and so are
      `pnpm lint`, `pnpm typecheck` and `pnpm test` in `web/`.
- [ ] The Rust, Node and pnpm pins in `ci.yml` and `.github/actions/setup-viewer`
      name the new versions, with their comments still true.
- [ ] `nix flake check` passes locally, the NixOS VM test included.
