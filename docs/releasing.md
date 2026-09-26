# Releasing

A release is one run of
[`release.yml`](../.github/workflows/release.yml), started by hand against a
version number, and nothing a human does around it. There is no commit to land
first and no tag to push: the workflow runs the checks, bumps the workspace
version and lands that on `main`, builds every artifact from the commit it just
made, runs each of them, tags that commit, publishes the Release, and records
what it published where the flake reads it. This page describes that file
rather than a procedure to follow.

## Coining one

In the Actions tab, **Release** → **Run workflow**, and type the version
without its `v` — `0.1.2`, or `0.1.0-rc.1`. Or from a terminal:

```console
$ gh workflow run release.yml -f version=0.1.2
```

The number is the one input a run has, and so the one thing about it that can
be a typo. The first job to touch it refuses anything that is not semver's own
shape, refuses a leading `v` — the `v` belongs to the tag and the workflow adds
it — and refuses a version whose tag already exists.

What `main` is at the moment you press the button is what gets released, so
that is the thing to have settled first: the run takes `main`'s head, and every
artifact is built from it.

## The order, and what a failure leaves

Each of these waits on the one above it.

1. **Tests.** The run calls `ci.yml` rather than asking GitHub whether those
   checks happened to have passed on whatever `main` had reached. A release is
   started at a moment nobody chose for its CI history, so the run that ships
   is the run that tests. Red here and the release stops having written
   nothing.
2. **Prepare.** The workspace version in [`Cargo.toml`](../Cargo.toml) is set
   to the release's — `Cargo.lock` with it, which carries a version per
   workspace crate — and committed to `main` by `github-actions[bot]` as
   `chore: version <version>`. Everything below is built from that commit by
   sha rather than from `main`, so an hour-long release and a branch that moves
   under it are not the same thing.
3. **The viewer**, built once. `rust-embed` reads `web/dist` at compile time
   and what vite writes does not vary by platform, so building it per leg would
   cost eight times over and let the legs disagree about what they embedded.
4. **Eight legs**, each building its artifact and then running it: five bare
   CLI binaries, and beside them one desktop app per desktop platform — the
   Linux one as `Verkstead-x86_64.AppImage`, the macOS one as
   `Verkstead-universal.dmg`, the Windows one as `Verkstead-x86_64.msi`.
5. **Publish.** The tag is made on the prepare commit, annotated, and pushed;
   then the Release is created under it with all eight artifacts and generated
   notes.
6. **Manifest.** [`nix/release.json`](../nix/release.json) is written from the
   published assets and committed to `main`, so the flake fetches what was just
   published.

**A run that fails leaves a `main` whose version has moved ahead of its last
release, and nothing else.** That is where `main` was going anyway, and running
the same version again finds the bump already made and carries on — the retry
is the same release rather than a new one. Nothing that cannot be taken back
happens until every artifact exists and has been run: the tag and the Release
are the last two steps, in that order, and a leg that will not build reaches
neither.

Two things follow from that ordering and are worth saying out loud. **The tests
run on the tree before the bump**, so what ships differs from what was tested
by a version string and nothing else; testing the bumped tree would mean
committing it first, and a red check would then have left that commit on
`main`. And **the version bump is one of two commits this workflow pushes
straight to `main`** with no pull request — see
[the git workflow](agents/git-workflow.md#exception-the-release-commits) for
why both are exceptions and what makes them safe.

## What the legs are, and what holds them to their floors

The CLI legs each run on a runner of their own architecture; the Linux desktop
leg runs on a plain `ubuntu-24.04` runner and compiles nothing at all. It waits
on the whole of the matrix above, takes the `verkstead-linux-x64` artifact that
matrix built — the static musl binary — puts it where
[`desktop/electron-builder.yml`](../desktop/electron-builder.yml) expects the
CLI this app carries, and packs the Electron app around it with `pnpm run pack`
([ADR-0020](adr/0020-electron-desktop.md)). Nothing is built twice: the sidecar
a downloader gets is the very file the CLI leg published beside it.

Which is also what moved that artifact's floor. A bundle carries the libraries
it links but not the C runtime, so a downloader's loader has to satisfy the
highest glibc anything inside the file names — and where that used to be the
image the leg compiled in, it is now Electron's own, the musl CLI naming no
glibc at any version. So the number is stated rather than inherited: the leg
reads every versioned reference out of the mounted image with `objdump` and
fails on anything above the 2.25
[adoption.md](adoption.md#the-desktop-app-on-a-linux-machine) promises.
Whichever way round it is written, the promise and the check are one number —
move Electron and both move.

The macOS desktop leg compiles nothing either, and it is the one runner that
packs for both Macs at once: `macos-15` is the Apple silicon image, and what it
builds there is the app rather than the CLI. It waits on the whole of the matrix
above and downloads both `verkstead-macos-arm64` and `verkstead-macos-x64` — the
two headless binaries that matrix built and ran — then hands the pair to `pnpm
run pack`, which `lipo`s them into the one universal sidecar the app carries
before electron-builder builds the x64 app and the arm64 app that
`@electron/universal` merges into `Verkstead-universal.dmg`
([ADR-0020](adr/0020-electron-desktop.md)). Both halves of the sidecar a
downloader gets are the very files the CLI legs published beside it, and the
`lipo` is the pack's rather than the leg's, so a developer packing from a
checkout packs the same way. A universal merge refuses an extra resource that
differs between the two halves, which a sidecar joined before either app is
built cannot. Its floor is written into the bundle rather than inherited from a
runner — `LSMinimumSystemVersion`, 12.0, which is Electron's own where the Rust
bundle's was the Apple silicon half's 11.0 — and it is the number
[adoption.md](adoption.md#the-desktop-app-on-a-mac) gives a downloader.

The Windows desktop leg is the one with an installer in it, and it is an
installer because a Windows install became two files:
[`tools/build-windows-msi.sh`](../tools/build-windows-msi.sh) builds the unified
`verkstead` with the `desktop` feature its default leaves on, and beside it the
windows-subsystem shim that supplies the `desktop` verb a Start-menu shortcut
has nowhere to write. Two files beside each other are not a portable download,
so [`tools/verkstead.wxs`](../tools/verkstead.wxs) wraps them in
`Verkstead-x86_64.msi` (ADR-0012, as amended). That package is per-user
throughout — the binaries under `%LOCALAPPDATA%\Programs\Verkstead`, the
shortcut in the user's own Start menu, the install directory appended to the
user's `PATH` — because the app is unsigned and elevation would buy a downloader
nothing they wanted. The WiX toolset that compiles it is the runner image's own.
There is no floor to hold any of it to: what an AppImage promises about glibc
and a bundle about macOS 12, an exe gets from the C runtime Windows itself
ships.

One thing about that package cannot say what the tag says. A Windows Installer
version is three numbers and nothing after them, so `v0.1.0-rc.1` and
`v0.1.0-rc.2` both arrive in Apps & Features as `0.1.0` — which is why the
package allows an upgrade from a version reading the same as its own, and the
second rc replaces the first rather than standing beside it.

Each desktop leg then asserts the artifact itself rather than what was lying
beside it: the AppImage is run as the file that is uploaded, the dmg is mounted
and run out of the mount, and the msi is installed and everything afterwards
asked of the install it left. What they assert is the same three things — it
starts, it serves a document with the viewer's bundle named in it, and its own
log says an icon went up — and each of them is bounded, because the failures
these apps draw are dialogs and a dialog nobody dismisses would hold a runner
for six hours. The Linux leg mounts the file before it reads anything out of it,
which is an assertion in its own right and the one the adoption documents make
to a downloader: the runtime packed into it carries its own squashfuse, so it
mounts on a machine with a `/dev/fuse` and no libfuse2 at all.

Every leg then asserts a fourth, and it is the one the running app cannot be
asked for: the half of the binary a *session* gets. In the AppImage and the
bundle that is the binary inside the artifact, run by path and asked for `ask`;
on Windows it is `verkstead guide` in a terminal opened after the install, which
is the same claim through the door an msi has — the `PATH` entry it wrote. An
artifact carrying the app alone would pass every assertion above it and hand
each session it spawned a binary with no `ask` in it. The Windows leg checks two
more that are the installer's own: the install is in the user's profile with its
record under `HKCU`, and the Start-menu entry opens the shim rather than the
console program beside it. The dmg's leg adds two of its own. That the app comes
back out of the image sealed, over the bundle and over the sidecar inside it:
the reason that was first written is gone — the bundle's executable was a shell
script, whose signature lives in an extended attribute a copy can drop, and it
is a real Mach-O again — but an Apple silicon Mac refuses to execute a Mach-O
carrying no signature at all, so a seal the pack or the image broke is the
"damaged" a Mac will not open rather than the "unidentified developer" it offers
a way past. And that the icon resolves: `CFBundleIconFile` read off the mounted
plist and `iconutil` opened on the file it names, the Finder icon being the one
thing about a bundle that nothing running the app can tell you about.

The manifest is the nix systems alone, and that is the one place a count is
still the right question: what the flake and the NixOS module run is the
headless daemon, so nothing fetches a desktop bundle through nix — and nothing
fetches the Windows CLI binary through it either, there being no nix system to
key one under. Four stays four however many assets a Release carries. The
desktop assets are checked by name instead, in `publish`, which is the one place
those names are written down, and the bare binaries are counted there: five of
them since the Windows port, held apart from the desktop artifacts by the
artifact name each leg uploaded under rather than by the name of the file inside
it.

A version with a hyphen in it — `0.1.0-rc.1` — is semver's own spelling of a
pre-release, and the workflow marks the Release as one. That is the difference
between a release that ships and one that only rehearses the pipeline: GitHub
keeps a pre-release off `releases/latest`, which is the url an install command
asks for. It is also the only difference, the run being the same run either
way, so a rehearsal really does rehearse.

The compiled binary drops the hyphen and everything after it, for the reason
the msi does: `v0.1.0-rc.1` and `v0.1.0-rc.2` are both a `Cargo.toml` reading
`0.1.0`, which is what `verkstead --version` prints. The tag is the only place
the difference between two release candidates lives, and `prepare` is where
that truncation happens.

**The manifest on `main` names the last release**, which is what
`packages.verkstead` downloads — see the note in [`flake.nix`](../flake.nix),
which falls back to the source build on any system the manifest has no entry
for.

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
   means the version carried a hyphen.

2. **That binary, downloaded and run** somewhere `verkstead` is not already on
   the `PATH`. Then `verkstead --version`, which prints the version you typed,
   up to its hyphen.

3. **The AppImage, downloaded and run** on a Linux desktop — the same way, and
   made executable first because a Release asset carries no mode:

   ```console
   $ curl -fsSL -O \
       https://github.com/tobico/verkstead/releases/latest/download/Verkstead-x86_64.AppImage
   $ chmod +x Verkstead-x86_64.AppImage
   $ ./Verkstead-x86_64.AppImage
   ```

   There is no `--help` to ask this one for: it is an app rather than a CLI, and
   what says the file is what it claims to be is that it comes up — the
   workbench in a window of its own, and an icon in the tray beside it. Click
   through a Conversation or two, which is the part a runner under Xvfb cannot
   judge. Which release this is was step 2's question, and the bare binary
   answered it.

   A desktop with no tray host shows no icon and is serving all the same, and a
   machine with no `/dev/fuse` will not mount the file at all — both of them are
   [what a downloader is told](adoption.md#the-desktop-app-on-a-linux-machine).

4. **The dmg, downloaded in a browser and opened** on a Mac. In a browser
   deliberately: the `com.apple.quarantine` flag Gatekeeper reads is set by
   whatever did the downloading, and `curl` sets nothing — an app fetched with
   it opens with no refusal at all, and that refusal is the one part of this
   install no workflow can rehearse.

   Drag Verkstead into Applications, double-click it, and walk the three steps
   through System Settings → Privacy & Security that
   [a downloader is told](adoption.md#the-desktop-app-on-a-mac). The workbench
   comes up in a window of its own, with a tile in the Dock and an icon in the
   menu bar beside it.

   Then tick **Launch on Startup** on the settings page's **Desktop** section,
   log out and back in: Verkstead comes back with the icon in the menu bar and
   no window over what you were doing, and **Open** on that icon is the way to
   one. That is the one reading no runner can take — a job dies with its login
   session, and what the app reads is the launch event `loginwindow` alone
   sends.

5. **The msi, downloaded in a browser and opened** on a Windows machine. In a
   browser deliberately, for the reason the dmg is: the mark SmartScreen reads
   is put on the file by whatever downloaded it, and `curl` puts on nothing — a
   package fetched with it installs with no warning at all, and that warning is
   the one part of this install no workflow can rehearse.

   Walk past it the way
   [a downloader is told](adoption.md#the-desktop-app-on-windows), then open
   Verkstead from the Start menu. The viewer opens in the browser and the icon
   lands in the notification area — inside the flyout the `^` opens, until it
   is dragged out onto the taskbar. Then, in a terminal opened after the install
   rather than one that was already up, `verkstead --version`: the `PATH` entry
   is the half of this download a human at a terminal uses, and a terminal that
   was already open never read it.

6. **The flake, refreshed past nix's cache** — after the manifest commit has
   landed on `main`, which is a job later than the Release itself:

   ```console
   $ nix run --refresh github:tobico/verkstead#verkstead -- --version
   ```

   What it prints is the manifest's version, and so the tag's.

7. **The two commits on `main`**, both by `github-actions[bot]`: `chore:
   version <version>` from before the build, and `chore: release manifest for
   <tag>` from after it. The manifest names the new version and carries all
   four nix systems. Neither commit starts a CI run —
   [the git workflow](agents/git-workflow.md#exception-the-release-commits)
   records why they are the two writes to `main` that skip review, and what
   makes each safe.

8. **The Update Notice**, on a server still running the previous version: the
   Repo list gains a banner naming the new one, and the README's `## Updating`
   section is where its link lands — so that section has to exist by then. The
   server asks GitHub at startup and daily after, so restart the old server
   rather than waiting a day.
