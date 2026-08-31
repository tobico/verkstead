# 03. AppImage and the release legs

## Goal

A `v*` tag builds `Verkstead-x86_64.AppImage` alongside the four bare CLI
binaries, attaches it to the GitHub release, and the file runs the stage-02
tray on a stock distribution. Icon and desktop-entry assets exist once, for
every packaging stage after this one to reuse.

## Decisions in force

- **AppImage is the Linux desktop artifact of the first release**
  ([ADR-0012], grilling Q10): Flatpak is deferred — its sandbox refuses the
  nested namespaces bubblewrap needs, and the `flatpak-spawn --host` workaround
  stack is not worth shipping first; when it comes it is a `.flatpak` bundle on
  GitHub releases, not Flathub. The nix flake stays daemon-only (Q11).
- **x86_64 only**, per the Brief; arm64 Linux users have the bare CLI.
- **The desktop binary need not be musl-static.** The tray stack drags in
  system GUI libraries; dynamic-plus-bundled inside the AppImage is the
  ordinary answer, and the CLI legs stay musl exactly as they are.
- **Artifact naming follows the release's existing scheme** where it applies;
  the AppImage keeps the format's own `Name-arch.AppImage` convention.
- Release legs are added per packaging stage — this stage builds only the
  Linux one, plus the shared icon generation.

## Proposed tasks (provisional)

1. **Icons and desktop entry** — `.desktop` file with `net.tobico.Verkstead`,
   sized PNGs from `assets/icons/verkstead-hammer.png` via
   `tools/generate-icons.sh`'s approach. Accepts: assets referenced by the
   AppImage build; regeneration scripted, not hand-drawn.
2. **AppImage build** — script the bundle (appimagetool or linuxdeploy),
   runnable locally and in CI. Accepts: the file launches the tray on a
   runner-fresh distro image; `--no-open` smoke test headless.
3. **Release leg** — `release.yml` gains the desktop Linux job on the existing
   viewer artifact, with assertions in the spirit of the CLI legs' (launches,
   viewer embedded). Accepts: tag dry-run (`workflow_dispatch` or a test tag)
   attaches the AppImage.

## Re-verify at start

- The shape of `.github/workflows/release.yml` (four native-runner CLI legs, a
  shared viewer job) and whether the ubuntu-24.04 runner carries the GUI
  packages the tray build needs.
- What stage 02 settled about the Linux tray backend — appindicator vs ksni
  decides what must ride inside the AppImage.
- GitHub Actions billing on this account has blocked runs before; a leg that
  dies in seconds with no steps is billing, not the workflow.

[ADR-0012]: ../../adr/0012-desktop-tray-binary.md
