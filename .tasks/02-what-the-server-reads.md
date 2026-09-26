# 02. What the server reads of an AppImage

## What to build

The server has a reading of `$APPDIR` that was written for the Rust AppImage and
is wrong for this one, and the app hands its sidecar an environment the AppImage
runtime's own launcher wrote. Both go, and a real session started out of the
packed AppImage from task 01 is what says so.

**What the server does today.** Where the running image sits inside an absolute
`$APPDIR` and there is a `usr/lib` directory beside it, `crates/server/src/
sandbox.rs` reads that as an image packed with its own libraries: it writes a
two-line `/bin/sh` launcher that exports `LD_LIBRARY_PATH` and execs the image
behind it, puts that launcher where a session finds `verkstead` on its `PATH`,
and binds those libraries into every Sandbox. That was exactly right for a
dynamically linked GTK binary with no rpath. **Every condition of it is true of
an electron-builder AppImage and none of the intent is** — its `usr/lib` holds
the AppImage runtime's own libraries, and the sidecar is the static musl CLI,
which has no loader to point anywhere. So a session would be handed a shell
script in front of its binary and a bind of libraries nothing in it reads, and
anything the CLI shells out to — `git`, for a Set's project, branch and Diff —
would run with `LD_LIBRARY_PATH` naming a mount.

**Removed rather than gated.** There is no AppImage left that wants it once this
stage lands, and the Rust one's own release leg goes in the very next task, so
nothing ships in between. The tests that cover the reading go with it or turn
round to assert that an image inside an AppDir is handed over as itself.

**And the sidecar gets a clean environment.** The app spawns it with this
process's own, which inside the packed app is whatever the AppImage's launcher
exported — `LD_LIBRARY_PATH` at the bundle's libraries, `XDG_DATA_DIRS` and
`GSETTINGS_SCHEMA_DIR` at the bundle's share, and a `PATH` led by the AppDir.
Those are Electron's to run over and nobody else's: the server inherits them,
every session inherits them from the server, and a bundled library reaching a
host binary is the oldest way an AppImage breaks something outside itself. What
the app passes on is decided in a function of the environment it was given, so
the suite can pin it without spawning anything — the shape the rest of the main
process already takes for the sake of the lint wall.

## Acceptance criteria

- [ ] A session started under the packed AppImage is handed the sidecar itself:
      no launcher script on its `PATH`, and no libraries bound in from the mount.
- [ ] Nothing inside a session, and nothing the CLI spawns, carries an
      `LD_LIBRARY_PATH`, `XDG_DATA_DIRS` or `GSETTINGS_SCHEMA_DIR` out of the
      AppImage — and a `verkstead ask` from inside a session still answers.
- [ ] `cargo test` is green with the old reading's tests gone or turned round,
      and the desktop suite pins what the sidecar's environment is stripped of.
