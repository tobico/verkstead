# 03. The release leg, and the Rust AppImage retired

## What to build

`desktop-linux` becomes Electron's leg, and the script it used to call is
deleted in the same slice — a leg that no longer runs it and a script nothing
runs are one change.

**The leg waits on the CLI matrix now**, which is what makes the sidecar the
released artifact rather than a second build of it (ADR-0020): it takes the
`verkstead-linux-x64` artifact — the static musl binary — puts it where task
01's configuration expects the CLI, and packs. A `needs` cannot name one row of
a matrix, so it waits on the whole `build` job.

**What goes with the Rust build.** There is nothing left on this leg to compile:
no Rust toolchain, no cache of its own, no `libgtk-3-dev`, and no `ubuntu:22.04`
container — which existed only to hold a GTK binary to a glibc floor, and which
had to be handed `/dev/fuse`, `SYS_ADMIN` and a way past the host's apparmor
profile to mount what it built. A plain `ubuntu-24.04` runner has FUSE and a
`/dev/fuse` already, and electron-builder brings its own `mksquashfs`, so the
apt step goes down to what a run under a screen wants.

**The floor stays, read off the artifact rather than stated** (Set 885 Q4b). The
musl CLI has no glibc floor at all, so what a downloader's loader has to satisfy
is Electron's own — a number nobody has measured, and one the dev shell's
Electron cannot answer for, being built against nixpkgs' glibc. So the step
reads every versioned glibc reference out of the packed app as the old one did,
and holds it to the number `docs/adoption.md` promises. Whichever way round it
is written, the promise and the check are one number: moving Electron moves both.

**The three assertions are on the file that is uploaded**, each bounded, because
the failures this app draws are dialogs and nobody is there to dismiss one.
Under Xvfb, because the window is the point. They are: it serves a document with
the viewer's hashed bundle named in it; the binary inside the mounted image
answers `ask`, which is the half of this download a *session* gets and the
invariant the Sandbox stands on; and the app's own log says the window opened and
the icon went into the tray. The log is where the last one lives because neither
is visible from outside the process. **The tray line is written whatever the bus
does** — stage 03 measured the app saying the icon is in the tray with no
watcher on the bus at all — so the assertion stands on a runner with no session
bus; a `dbus-run-session` around the run is what would make it mean what stage 03
measured, and is this task's judgement.

**The artifact name and the file name both stay**: `publish` fetches desktop
artifacts by a `desktop-*` pattern so they are never counted among the five CLI
binaries, and it names `Verkstead-x86_64.AppImage` in the one place those names
are written down. Neither changes, which is the point — nothing downstream of
this leg knows the file was made differently.

**Then the Rust AppImage goes**: `tools/build-appimage.sh`, and out of the dev
shell whatever only it wanted. GTK and `pkg-config` stay — the crate still
builds until stage 08. `docs/development.md`'s build list loses the line that
names the script, because it names a file that is no longer there; the paragraph
describing what the AppImage *is* is task 04's to rewrite.

**What stands as proof** (Set 885 Q4c). `release.yml` cannot be rehearsed
against this branch: it is dispatch-only, and `prepare` checks out `main`
whatever ref the run was started against and pushes a version commit to it — so
a dispatch from here would build main's tree and run main's leg. So the leg is
proven by `actionlint` over the workflow and by running its steps by hand in an
`ubuntu-24.04` container against a CLI built from this branch, end to end.

## Acceptance criteria

- [ ] The leg's steps, run by hand in an `ubuntu-24.04` container against this
      branch's musl CLI, pack the app and pass all four assertions — and each of
      them fails, naming what went wrong, when broken on purpose.
- [ ] `publish` is untouched and still right: `desktop-linux` arrives carrying
      `Verkstead-x86_64.AppImage`, and the CLI count is still five.
- [ ] Nothing in the repository builds an AppImage of the Rust tray any more, and
      `actionlint`, `nix flake check` and CI are green.
