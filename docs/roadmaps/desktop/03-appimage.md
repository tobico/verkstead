# 03. AppImage and the release legs

## Goal

A `v*` tag builds `Verkstead-x86_64.AppImage` alongside the four bare CLI
binaries, attaches it to the GitHub release, and the file runs the stage-02
tray on a stock distribution. Icon and desktop-entry assets exist once, and so
does the release plumbing that carries a desktop artifact to a Release, for
stages 04 and 05 to reuse.

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
- **The shared release plumbing is this stage's, laid once.** A leg does not
  attach anything: the four CLI legs upload workflow artifacts and one
  `publish` job collects them, counts them and creates the Release. Carrying a
  desktop artifact through it — the download pattern, the count assertion, the
  `needs`, the upload glob — is written here, so that stages 04 and 05 add a
  build leg and an artifact name and touch `publish` no further. Which is why
  this stage runs first of the three.
- **The packaging assets live outside `assets/`.** `web/vite.config.ts` sets
  `publicDir: "../assets"`, so everything under it is copied untouched into the
  viewer build — and the viewer is embedded in every binary, the headless CLI
  included. A `.desktop` file and a set of packaging icons served at the web
  root and carried inside the CLI is neither's business, so they get a
  directory of their own, made by the first packaging stage to run and reused
  by the other two — which by the ordering above is this one. The one piece of
  artwork stays where it is: `assets/icons/verkstead-hammer.png` is the
  viewer's own source too.
- Release legs are added per packaging stage — this stage builds only the
  Linux one, plus the shared icon generation.

## Proposed tasks (provisional)

1. **Icons and desktop entry** — `.desktop` file with `net.tobico.Verkstead`
   and the sized PNGs the packaging stages need, generated from
   `assets/icons/verkstead-hammer.png` the way `tools/generate-icons.sh`
   generates the viewer's, into a packaging directory of their own rather than
   into `assets/`. Accepts: assets referenced by the AppImage build;
   regeneration scripted, not hand-drawn; nothing desktop-only is served by the
   viewer or embedded in the CLI.
2. **AppImage build** — script the bundle (appimagetool or linuxdeploy),
   runnable locally and in CI. Accepts: the file launches the tray on a
   runner-fresh distro image; `--no-open` smoke test headless.
3. **Release leg** — `release.yml` gains the desktop Linux job on the existing
   viewer artifact, with assertions in the spirit of the CLI legs' (launches,
   viewer embedded). Accepts: tag dry-run (`workflow_dispatch` or a test tag)
   attaches the AppImage.
4. **The publish job takes desktop artifacts** — `publish` fetches with
   `pattern: verkstead-*`, asserts it found exactly four ("Expected four
   binaries", which fails the release), uploads `binaries/verkstead-*` and
   `needs: build` alone. `Verkstead-x86_64.AppImage` matches none of that, and
   stage 05's `.exe` would match the pattern and break the count. Widen it
   once, for all three artifacts rather than for this one. Accepts: the CLI
   binaries are still counted as a set of their own, so a missing one still
   fails; a desktop artifact that was built and not attached fails too;
   `publish` waits on the desktop legs.
5. **Docs** — `docs/releasing.md` still says the workflow "publishes the four";
   it names what a tag now produces. And the install story gains its Linux
   desktop half: `README.md` and `docs/adoption.md` both tell it as the flake
   and nothing else, while the AppImage is now the other way onto a Linux
   machine — and the flake stays daemon-only (Q11), so the two are told apart
   rather than merged. Accepts: no count in `releasing.md` that the workflow
   contradicts; a Linux reader can find the AppImage and can tell which of the
   two they want.

## Re-verify at start

- The shape of `.github/workflows/release.yml` — four native-runner CLI legs, a
  shared viewer job, one `publish` job that creates the Release and a
  `manifest` job that hashes the four CLI assets for the flake, which is the
  one place four is the right number and stays so. And whether the ubuntu-24.04
  runner carries the GUI packages the tray build needs, against what stage 02
  settled for CI.
- What stage 02 settled about the Linux tray backend — appindicator vs ksni
  decides what must ride inside the AppImage.
- What the viewer copies whole when the stage starts (`publicDir` in
  `web/vite.config.ts`) — what the packaging assets have to stay out of.
- GitHub Actions billing on this account has blocked runs before; a leg that
  dies in seconds with no steps is billing, not the workflow.

[ADR-0012]: ../../adr/0012-desktop-tray-binary.md
