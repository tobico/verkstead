# Releasing

A release is a tag and nothing else.
[`release.yml`](../.github/workflows/release.yml) fires on `v*`: it builds the
viewer once, then one binary per platform on a runner of that platform's own
architecture, runs each binary it built, publishes the four as a GitHub Release
under the tag, and finally commits [`nix/release.json`](../nix/release.json) to
`main` so the flake fetches what was just published. None of that is
hand-driven, and nothing in it is hand-edited afterwards.

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

3. **The flake, refreshed past nix's cache** — after the manifest commit has
   landed on `main`, which is a job later than the Release itself:

   ```console
   $ nix run --refresh github:tobico/verkstead#verkstead -- --version
   ```

   What it prints is the manifest's version, and so the tag's.

4. **The manifest on `main`** names the new version and carries all four nix
   systems, committed by `github-actions[bot]` as
   `chore: release manifest for <tag>`. That commit deliberately starts no CI
   run — [the git workflow](agents/git-workflow.md#exception-the-release-manifest)
   records why it is the one write to `main` that skips review.

5. **The Update Notice**, on a server still running the previous version: the
   pending list gains a banner naming the new one, and the README's `## Updating`
   section is where its link lands — so that section has to exist by then. The
   server asks GitHub at startup and daily after, so restart the old server
   rather than waiting a day.
