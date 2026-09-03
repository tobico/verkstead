# 03. The Linux artifacts

## What to build

The AppImage bundles the unified binary — built with the feature on — and its
`AppRun` execs `verkstead desktop`. The release workflow's musl legs build the
CLI with the feature off so the released CLI stays the static binary it is
today, and their dependency-tree check is what holds GUI dependencies out of
it from now on — GTK will not link against musl, so a creep fails the leg
loudly. The nix source package builds slim the same way: it is the daemon the
NixOS module runs headless, and its closure gains nothing.

## Acceptance criteria

- [ ] The AppImage runs the tray app, and the binary inside it answers `ask` —
      with the release leg's symbol-floor check following the new binary name.
- [ ] The released CLI is still a static musl binary with no GTK anywhere in
      its dependency tree, and the tree check refuses one that grows it.
- [ ] The nix package builds without the desktop feature and its closure
      carries no GTK.
