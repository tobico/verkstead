# Releasing

A release is a tag and nothing else.
[`release.yml`](../.github/workflows/release.yml) fires on `v*`: it builds the
viewer once, then the bare CLI binary for each platform on a runner of that
platform's own architecture, and beside them the Linux desktop app as
`Verkstead-x86_64.AppImage`. Every leg runs what it built, all of it is
published as a GitHub Release under the tag, and finally the workflow commits
[`nix/release.json`](../nix/release.json) to `main` so the flake fetches what
was just published. None of that is hand-driven, and nothing in it is
hand-edited afterwards.

The CLI legs each run on a runner of their own architecture; the desktop leg
runs in an `ubuntu:22.04` container on top of one. That is the whole of what
decides the AppImage's floor — a bundle carries the libraries it links but not
the C runtime, so a downloader's loader has to satisfy the glibc the file was
compiled against, and 22.04's 2.35 reaches Ubuntu 22.04, Debian 12 and
everything above them. The leg reads the symbols back afterwards and fails on
anything higher, so the floor and the promise
[adoption.md](adoption.md#the-desktop-app-on-a-linux-machine) makes about it
cannot drift apart. Move the image and both move.

The manifest is the CLI binaries alone, and that is the one place a count is
still the right question: what the flake and the NixOS module run is the
headless daemon, so nothing fetches a desktop bundle through nix and four stays
four however many desktop artifacts a Release carries. The desktop assets are
checked by name instead, in `publish`, which is the one place those names are
written down.

A tag with a hyphen in it — `v0.1.0-rc.1` — is semver's own spelling of a
pre-release, and the workflow marks the Release as one. That is the difference
between a tag that ships and one that only rehearses the pipeline: GitHub keeps
a pre-release off `releases/latest`, which is the url an install command asks
for.

**Nothing has been released under this name yet.** The manifest on `main` ships
with its `systems` empty, which is why `packages.verkstead` is the source build
for now — see the note in [`flake.nix`](../flake.nix). The first run of this
workflow writes a real manifest and switches it over with nothing to undo.

## Before you tag

- **The version in [`Cargo.toml`](../Cargo.toml) matches the tag without its
  `v`.** Nothing checks this. The manifest takes its version from the tag while
  the binary reports the one it was compiled with, so a mismatch ships a binary
  that disagrees with the flake about what it is — and, where the tag is the
  higher of the two, an Update Notice naming an update that is already
  installed.
- **The commit is already on `main`.** The manifest job checks out `main` rather
  than the tag, so a tag on a branch publishes a Release whose manifest lands on
  a `main` that does not contain the code.
- **CI is green on that commit.** `release.yml` builds each binary and runs it,
  but it runs no tests; those are `ci.yml`'s, and `ci.yml` does not run on tags.

## Tagging

```console
$ git tag -a v0.1.0 -m 'Verkstead v0.1.0' <sha-on-main>
$ git push origin v0.1.0
```

## After the run

The workflow checks its own manifest: it re-downloads every published asset
through the urls the manifest records and fails if a hash disagrees. What is
left to check by hand is the part no workflow sees — the install story a
newcomer actually follows.

1. **`releases/latest` resolves to the new tag.**

   ```console
   $ curl -sSI -o /dev/null -w '%{http_code}\n' \
       https://github.com/tobico/verkstead/releases/latest/download/verkstead-linux-x64
   200
   ```

   A `404` means GitHub still has no release that is not a pre-release, which
   means the tag carried a hyphen.

2. **That binary, downloaded and run** somewhere `verkstead` is not already on
   the `PATH`. Then `verkstead --version`, which prints the tag without its `v`.

3. **The AppImage, downloaded and run** on a Linux desktop — the same way, and
   made executable first because a Release asset carries no mode:

   ```console
   $ curl -fsSL -O \
       https://github.com/tobico/verkstead/releases/latest/download/Verkstead-x86_64.AppImage
   $ chmod +x Verkstead-x86_64.AppImage
   $ ./Verkstead-x86_64.AppImage --version
   ```

   Then run it with no arguments: it serves, opens the viewer in the browser,
   and puts an icon in the tray. A desktop with no tray host shows no icon and
   is serving all the same, which is
   [what a downloader is told](adoption.md#the-desktop-app-on-a-linux-machine).

4. **The flake, refreshed past nix's cache** — after the manifest commit has
   landed on `main`, which is a job later than the Release itself:

   ```console
   $ nix run --refresh github:tobico/verkstead#verkstead -- --version
   ```

   What it prints is the manifest's version, and so the tag's.

5. **The manifest on `main`** names the new version and carries all four nix
   systems, committed by `github-actions[bot]` as
   `chore: release manifest for <tag>`. That commit deliberately starts no CI
   run — [the git workflow](agents/git-workflow.md#exception-the-release-manifest)
   records why it is the one write to `main` that skips review.

6. **The Update Notice**, on a server still running the previous version: the
   Repo list gains a banner naming the new one, and the README's `## Updating`
   section is where its link lands — so that section has to exist by then. The
   server asks GitHub at startup and daily after, so restart the old server
   rather than waiting a day.
