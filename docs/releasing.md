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

The number is the one input a run must have, and so the one thing about it
that can be a typo. The first job to touch it refuses anything that is not semver's own
shape, refuses a leading `v` — the `v` belongs to the tag and the workflow adds
it — and refuses a version whose tag already exists.

What `main` is at the moment you press the button is what gets released, so
that is the thing to have settled first: the run takes `main`'s head, and every
artifact is built from it.

**The note** is the other input, and it is optional: one line, put above the
notes GitHub generates, and left empty the notes are those alone.

```console
$ gh workflow run release.yml -f version=0.3.0 \
    -f note='On Windows, install this exe once by hand over the earlier Windows Installer package. It removes that package first.'
```

It is for what somebody already running the last Release has to do by hand to
take this one, which the generated list of what landed never says. Most
releases have nothing of the kind and leave it empty. The first release whose
Windows download is `Verkstead-x86_64.exe` is the one that needs it, with the
line above: a machine holding the Windows Installer package earlier Releases
shipped is not updated across to the exe, and installing the exe over it once
is what moves it.

## The Mac's signing secrets

A Release's Mac app is signed with a Developer ID and notarized
([ADR-0027](adr/0027-cluster-updates.md), *The release*): Squirrel.Mac, which
the app updates itself through, refuses an ad-hoc signed update. Both need
things only the maintainer holds, kept as five **repository** secrets —
repository rather than an environment's, so that a throwaway workflow on
another branch can be handed the same certificate. **A run missing any of the five fails its
Mac leg before it packs**, naming each one it has not got, and publishes
nothing; a pack anywhere else — a checkout, a pull
request, a fork — has no secrets and signs ad-hoc, saying so in its log.

Made once, in this order.

1. **Enrol in the Apple Developer Program as an individual**, at
   [developer.apple.com/programs/enroll](https://developer.apple.com/programs/enroll/),
   with an Apple Account that has two-factor authentication on. An individual
   enrolment needs no company and no D-U-N-S number; the name a downloader
   is shown as the developer is your legal name. It is paid yearly, and the
   enrolment holder is the Account Holder, the only role that can make the
   certificate below.

2. **Make a Developer ID Application certificate**, on a Mac. In Keychain
   Access, **Keychain Access → Certificate Assistant → Request a Certificate
   From a Certificate Authority**, with your email, *Saved to disk*. Then at
   [Certificates, Identifiers & Profiles](https://developer.apple.com/account/resources/certificates/list),
   **+**, **Developer ID Application**, upload that request, download the
   `.cer` and open it, which puts it in the login keychain beside the private
   key the request made. *Developer ID Application* — not *Installer*, which
   signs `.pkg` files, and not *Apple Development*, which no Mac outside your
   team trusts.

3. **Export it as a `.p12`.** In Keychain Access under **My Certificates**,
   the row reading *Developer ID Application: <your name> (<team id>)* —
   expanded, it has the private key under it — **Export**, as *Personal Information Exchange (.p12)*, under a
   password of its own. That file and its password are the whole of the
   signing identity: keep the file somewhere other than this machine's
   Downloads, and the password in your password manager.

   **Export the certificate's row, not the key's.** The `.p12` has to hold
   the certificate *and* its private key: exported from the key alone it holds
   no identity, and the pack fails saying it found none.

4. **Make an App Store Connect API key**, which is what notarization
   authenticates with. At
   [App Store Connect → Users and Access → Integrations](https://appstoreconnect.apple.com/access/integrations/api),
   **Team Keys**, **+**, any name, the *Developer* access. Download the
   `AuthKey_<key id>.p8` — it can be downloaded once, and a lost one is
   revoked and made again — and note the **Key ID** on its row and the
   **Issuer ID** above the list.

5. **Put the five into the repository's secrets.** From a checkout, with
   `gh` signed in as somebody who can administer the repository:

   ```console
   $ base64 -i Certificates.p12 | gh secret set CSC_LINK
   $ gh secret set CSC_KEY_PASSWORD             # the .p12's password, typed at the prompt
   $ gh secret set APPLE_API_KEY < AuthKey_ABC123DEFG.p8
   $ gh secret set APPLE_API_KEY_ID --body ABC123DEFG
   $ gh secret set APPLE_API_ISSUER --body 01234567-89ab-cdef-0123-456789abcdef
   ```

   | Secret | What it holds |
   | --- | --- |
   | `CSC_LINK` | The `.p12`, base64-encoded |
   | `CSC_KEY_PASSWORD` | The password the `.p12` was exported under |
   | `APPLE_API_KEY` | The contents of the `.p8` |
   | `APPLE_API_KEY_ID` | The key's Key ID |
   | `APPLE_API_ISSUER` | The team's Issuer ID |

   The names are electron-builder's own: the two `CSC_` ones are what it reads
   a certificate from. The leg writes `CSC_LINK` and `APPLE_API_KEY` out to
   files in the runner's temp and hands electron-builder each file's path under
   the same name, so neither secret is ever handed over as itself.

A Developer ID certificate lasts five years and the API key until it is
revoked. Renewing either is steps 2–3 or step 4 again and the same `gh secret
set`; nothing in the repository names them.

## The order, and what a failure leaves

Each of these waits on the one above it.

1. **Tests.** The run calls `ci.yml` rather than asking GitHub whether those
   checks happened to have passed on whatever `main` had reached. A release is
   started at a moment nobody chose for its CI history, so the run that ships
   is the run that tests. Red here and the release stops having written
   nothing.
2. **Prepare.** The workspace version in [`Cargo.toml`](../Cargo.toml) is set
   to the release's — `Cargo.lock` with it, which carries a version per
   workspace crate, and [`desktop/package.json`](../desktop/package.json)
   beside them, that being where electron-builder reads the Windows
   installer's version and nothing else bumps it — and committed to `main` by
   `github-actions[bot]` as `chore: version <version>`. Everything below is
   built from that commit by sha rather than from `main`, so an hour-long
   release and a branch that moves under it are not the same thing.
3. **The viewer**, built once. `rust-embed` reads `web/dist` at compile time
   and what vite writes does not vary by platform, so building it per leg would
   cost eight times over and let the legs disagree about what they embedded.
4. **Eight legs**, each building its artifact and then running it: five bare
   CLI binaries, and beside them one desktop app per desktop platform — the
   Linux one as `Verkstead-x86_64.AppImage`, the macOS one as
   `Verkstead-universal.dmg` with `Verkstead-universal.zip` beside it, the
   Windows one as `Verkstead-x86_64.exe`. Each desktop leg uploads what
   electron-updater reads beside its files too — see
   [the update metadata](#the-update-metadata).
5. **Publish.** The tag is made on the prepare commit, annotated, and pushed;
   then the Release is created under it with every leg's artifacts and
   generated notes, the note above them where the run was given one.
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

## The update metadata

**A Release is also where a packed app finds its next version**
([ADR-0027](adr/0027-cluster-updates.md), *The release*).
[`desktop/electron-builder.yml`](../desktop/electron-builder.yml) names a
`github` provider for this repository, which is what has each pack write an
`app-update.yml` into the app — where electron-updater is to look — and a
`latest*.yml` beside the files it made: `latest-linux.yml` naming the
AppImage, `latest-mac.yml` naming the zip and the dmg, and `latest.yml` naming
the installer, each with the file's size and sha512. The Mac's two files and
the installer have a `.blockmap` beside them as well, for a differential
download; an AppImage carries its blockmap inside itself.

**Nothing is published by electron-builder.** `pnpm run pack` says
`--publish never` on every pack, and the metadata and the blockmaps go up with
the rest of the Release through the one `gh release create` — the publish job
names every one of them, both ways round, as it names the installers.

**The metadata is unsigned**, by decision: the pinned electron-builder has no
manifest signing, so what an update rests on is the sha512 in the file,
fetched over TLS from the Release, and on a Mac the Developer ID besides. So
each desktop leg reads its own metadata back against the files it is about to
upload — the names the Release will hold them under, the sizes and the hashes
— and reads the packed app's `app-update.yml` and the electron-updater inside
its `app.asar`, with
[`desktop/scripts/updates.mjs`](../desktop/scripts/updates.mjs). Only a stable
Release is ever an update: electron-updater reads `releases/latest`, which a
pre-release is never.

**And the Linux leg updates an app with it, against a feed of its own.**
[`desktop/scripts/update-feed.mjs`](../desktop/scripts/update-feed.mjs) packs
the app twice more, at 0.0.1 and 0.0.2 around the same sidecar, serves the
higher one's `latest-linux.yml` and AppImage from a `generic` feed on loopback,
starts the lower one pointed at it with `VERKSTEAD_UPDATE_FEED` — an override
only a test sets — and presses Update through the API. It passes once the row
has read *updating…* with the app's reports on it, the server has gone and come
back with no Update going, the image on disk is the one the feed offered, and
the relaunched app's log says 0.0.2 started, waited out the app it replaced,
and met neither its lock nor its listener. Before that, a failing run: an
install remembering an Update to 0.0.9 starts with the trouble line saying it
didn't land, then Update is pressed against the feed's `corrupt/`, an image
with one byte changed, and the row has to leave *updating…* for a trouble line
naming the sha512 mismatch while the server stays up on its image. Run it locally the same way: `pack
<sidecar> <dir>`, then `run <dir>` with a `DISPLAY` and port 8422 free. What it
cannot prove is the GitHub half — `app-update.yml` pointing at a Release.

**That half is first run at the release after the first one carrying
electron-updater.** The first such Release can only be updated *from*: every
install before it has no updater in it, so nothing takes it as an update. And
a rehearsal cannot stand in for the next one, because a rehearsal's tag is a
pre-release, which `releases/latest` never is and the updater never offers. So
once the second such Release is out, an AppImage of the first, run on a Linux
desktop and updated from its Devices row, is the check: the row reads
*updating…* and then *restarting…*, and the app comes back on the new version.

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
fails on anything above 2.25, Electron's own floor. Whichever way round it is
written, the floor and the check are one number — move Electron and both move.

The macOS desktop leg compiles nothing either, and it is the one runner that
packs for both Macs at once: `macos-15` is the Apple silicon image, and what it
builds there is the app rather than the CLI. It waits on the whole of the matrix
above and downloads both `verkstead-macos-arm64` and `verkstead-macos-x64` — the
two headless binaries that matrix built and ran — then hands the pair to `pnpm
run pack`, which `lipo`s them into the one universal sidecar the app carries
before electron-builder builds the x64 app and the arm64 app that
`@electron/universal` merges into `Verkstead-universal.dmg`
([ADR-0020](adr/0020-electron-desktop.md)). Before it packs, the leg checks that all five of [the Mac's signing
secrets](#the-macs-signing-secrets) are there, and a release with any of them
missing stops at that step. With them, the pack signs the app and the sidecar
inside it with the Developer ID under the hardened runtime, has Apple's notary
accept the app, and staples the ticket to it before either target is made — so
`Verkstead-universal.dmg` and `Verkstead-universal.zip` hold the same stapled
app, the dmg for a human to download and the zip for an update to install.
Both halves of the sidecar a downloader gets are the very files the CLI legs
published beside it, and the
`lipo` is the pack's rather than the leg's, so a developer packing from a
checkout packs the same way. A universal merge refuses an extra resource that
differs between the two halves, which a sidecar joined before either app is
built cannot. Its floor is written into the bundle rather than inherited from a
runner — `LSMinimumSystemVersion`, 12.0, which is Electron's own where the Rust
bundle's was the Apple silicon half's 11.0.

The Windows desktop leg compiles nothing either, and it is the one with an
installer in it. It waits on the whole of the matrix above and downloads
`verkstead-windows-x64.exe` — the binary that matrix built and ran — then hands
it to `pnpm run pack`, which packs the Electron app around that sidecar and
wraps the result in `Verkstead-x86_64.exe` through electron-builder's `nsis`
target ([ADR-0027](adr/0027-cluster-updates.md)), the one electron-updater
can update. It stays on `windows-2025` all the same: makensis is a Windows
program, and the install and every assertion after it want a real Windows to
happen on. That installer is per-user throughout and one-click, with no wizard
and no administrator asked for — the app under
`%LOCALAPPDATA%\Programs\Verkstead`, the shortcut in the user's own Start menu,
its record under `HKCU`, and the CLI's own directory inside the install
appended to the user's `PATH` — because the app is unsigned
([ADR-0020](adr/0020-electron-desktop.md#amended-the-windows-build-stays-unsigned)
says why) and elevation would buy a downloader nothing they wanted. That
directory rather than the install root, which holds a launcher named for the
product: Windows resolves a lookup without regard to case, and a root on
`PATH` would answer `verkstead guide` with a window. The directory's name and
the `PATH` entry are what
[`desktop/build/installer.nsh`](../desktop/build/installer.nsh) adds to
electron-builder's template, the include script the configuration names: the
target puts a per-user install under the package's name otherwise, and has no
key for a `PATH` entry at all. Nothing here installs NSIS either —
electron-builder fetches its own, which is one fewer unpinned tool than the leg
that compiled Rust on this runner had. There is no floor to hold any of it to:
what an AppImage promises about glibc and a bundle about macOS 12, an exe gets
from the C runtime Windows itself ships.

Each desktop leg then asserts the artifact itself rather than what was lying
beside it: the AppImage is run as the file that is uploaded, the dmg is mounted
and run out of the mount, and the exe is installed silently, with `/S`, and
everything afterwards asked of the install it left. What they assert is the
same three things — it starts, it serves a document with the viewer's bundle named in it, and its own
log says an icon went up — and each of them is bounded, because the failures
these apps draw are dialogs and a dialog nobody dismisses would hold a runner
for six hours. The Linux leg mounts the file before it reads anything out of it,
which is an assertion in its own right and a promise to a downloader: the
runtime packed into it carries its own squashfuse, so it
mounts on a machine with a `/dev/fuse` and no libfuse2 at all.

Every leg then asserts a fourth, and it is the one the running app cannot be
asked for: the half of the binary a *session* gets. In the AppImage and the
bundle that is the binary inside the artifact, run by path and asked for `ask`;
on Windows it is `verkstead guide` in a terminal opened after the install, which
is the same claim through the door an installer has — the `PATH` entry it
wrote. An artifact carrying the app alone would pass every assertion above it
and hand each session it spawned a binary with no `ask` in it. The Windows leg
checks more that are the installer's own. The install is in the user's profile
with its record under `HKCU` and none under `HKLM`. The Start-menu entry opens
the launcher at the install root rather than the console program inside it, and
opens no console with it — a subsystem being a fact about the file, read out of
its PE header rather than off a run. **The exe installed over itself leaves one
row in Installed apps, one `PATH` entry and one Start-menu entry**, which is
the shape electron-updater's update takes and the check that the include
script's `PATH` addition never doubles. **And installed over the last
Release's Windows Installer package, it leaves the same counts and no package**:
the include script finds that package by the UpgradeCode every earlier Release
installed under, and removes it before extracting anything, and the leg reads
Windows Installer's own record for this user afterwards and finds nothing, with
the Run value Launch on startup wrote still naming the launcher. Where the last
Release carries no such package that half is skipped, with a warning saying in
as many words that it went unchecked — every run after the first Release that
ships the exe alone, at which point the check and the removal retire together.
Last, **the uninstaller takes it all away and leaves the Data Directory**: run
the way Installed apps would run it, it removes the install directory, the
Start-menu entry, the row and the `PATH` entry, and the Data Directory is still
there afterwards. The dmg's leg adds five of its own, and every one of them is
about something a run of the app cannot report.

That the app comes back out of the image sealed, over the bundle and over the
sidecar inside it: the reason that was first written is gone — the bundle's
executable was a shell script, whose signature lives in an extended attribute a
copy can drop, and it is a real Mach-O again — but an Apple silicon Mac refuses
to execute a Mach-O carrying no signature at all, so a seal the pack or the
image broke is the "damaged" a Mac will not open rather than the "unidentified
developer" it offers a way past — and that the seal is a Developer ID
Application's under the hardened runtime, over both, since notarization and
Squirrel.Mac refuse anything less. That Gatekeeper accepts the app as a Mac
that has never seen it would, out of the mounted dmg and out of the zip
unpacked with `ditto` the way Squirrel.Mac unpacks one: `codesign --verify
--strict`, `xcrun stapler validate` for the ticket a Mac offline at first
launch has only the staple of, and `spctl --assess`, Gatekeeper's own verdict.
That the icon resolves: `CFBundleIconFile` read
off the mounted plist and `iconutil` opened on the file it names, the Finder
icon being what a downloader drags into Applications rather than anything the
process draws. That the bundle is still a Dock app and still promises the macOS
the words do: `LSUIElement` read back for its *absence*, which is the whole of
what closing the window to the Dock rests on, and `LSMinimumSystemVersion` read
back against the 12.0 floor — the same shape as the AppImage's glibc ceiling above, and moving
Electron moves both numbers. And that the menu bar's artwork came along at both
scales: the app hands Electron the one-times path and AppKit finds the `@2x`
beside it, so a Retina Mac is the only place a missing second file shows, and a
missing file of either size is an empty status item behind a log line still
saying the icon went up.

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

The compiled binary drops the hyphen and everything after it, and so does
every installer's version: a pre-release rehearses the release of its number,
so `v0.1.0-rc.1` and `v0.1.0-rc.2` are both a `Cargo.toml` reading `0.1.0`,
which is what `verkstead --version` prints and what **Installed apps** says. The tag is the only place
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
   machine with no `/dev/fuse` will not mount the file at all — neither is a
   failure of the release.

4. **The dmg, downloaded in a browser and opened** on a Mac. In a browser
   deliberately: the `com.apple.quarantine` flag Gatekeeper reads is set by
   whatever did the downloading, and `curl` sets nothing — an app fetched with
   it opens with no question at all, and what Gatekeeper makes of a
   quarantined download is the one part of this install no workflow can
   rehearse.

   Drag Verkstead into Applications and double-click it. The app is signed
   and notarized, so macOS asks once whether to open an app downloaded from
   the Internet, naming Verkstead, and nothing more: click **Open**, and the
   workbench comes up in a window of its own, with a tile in the Dock and an
   icon in the menu bar beside it. **A refusal that sends you to System
   Settings for *Open Anyway* is a failed release**, whatever the run said:
   it means the download Gatekeeper saw is not the notarized app the leg
   assessed.

   Then tick **Launch on startup** on the settings page's **Desktop** section,
   log out and back in: Verkstead comes back with the icon in the menu bar and
   no window over what you were doing, and **Open** on that icon is the way to
   one. That is the one reading no runner can take — a job dies with its login
   session, and what the app reads is the launch event `loginwindow` alone
   sends.

5. **The exe, downloaded in a browser and opened** on a Windows machine. In a
   browser deliberately, for the reason the dmg is: the mark SmartScreen reads
   is put on the file by whatever downloaded it, and `curl` puts on nothing — an
   installer fetched with it runs with no warning at all, and that warning is
   the one part of this install no workflow can rehearse.

   Walk past it the way
   [a downloader is told](../README.md#status). The install
   ends with Verkstead already running: the workbench in a window of its own,
   with a button on the taskbar and an icon in the notification area — inside
   the flyout the `^` opens, until it is dragged out onto the taskbar. Then, in
   a terminal opened after the install rather than one that was already up,
   `verkstead --version`: the `PATH` entry is the half of this download a human
   at a terminal uses, and a terminal that was already open never read it.

   Then tick **Launch on startup** on the settings page's **Desktop** section,
   sign out and back in: Verkstead comes back with the icon in the notification
   area and no window over what you were doing, and **Open** on that icon is the
   way to one. That is the reading no runner can take: the value under the
   Run key is read back where it is written, and what nothing tests until
   somebody signs in is Windows running the command line it holds.

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
