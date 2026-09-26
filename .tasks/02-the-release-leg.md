# 02. The release leg, and the Rust dmg retired

## What to build

`desktop-macos` becomes Electron's leg, and `tools/build-macos-dmg.sh` is
deleted in the same slice — a leg that no longer runs the script and a script
nothing runs are one change, which is how stage 05 retired the Rust AppImage's.

**It waits on the CLI matrix now**, as `desktop-linux` does, and that is what
makes the sidecar the released artifact rather than a second build of it
(ADR-0020): it downloads `verkstead-macos-x64` and `verkstead-macos-arm64` —
the two binaries the matrix already builds and runs — puts the executable bit
back on each, because an artifact is a zip and a zip carries no mode, and hands
both to `pnpm run pack`. A `needs` cannot name one row of a matrix, so it names
the whole `build` job.

**And it compiles nothing.** No Rust toolchain, no second cache, no
`targets: aarch64-apple-darwin, x86_64-apple-darwin`, and no `touch` of the
viewer's embed to defeat cargo's staleness — the viewer is already inside the
binaries the matrix built. The runner stays `macos-15`: `lipo`, `hdiutil` and
`codesign` are a Mac's, and an Apple-silicon image is the half more people
download.

**The assertions are on the file that is uploaded**, mounted with `hdiutil
attach -readonly -nobrowse` at a path the job names, and each of them bounded —
the failures this app draws it has a screen for are drawn as dialogs, and a
dialog nobody dismisses would hold the job until the runner's own six hours were
up. What they are:

- **The app serves the workbench**, run out of the mount with the Workbench Key
  written into a Data Directory beforehand, and the document names the viewer's
  hashed bundle. The app takes no flags — there is no `--no-open`, no `--listen`
  and no `--data-dir` on it — so `VERKSTEAD_DATA_DIR` is how the directory is
  said, which is the server's own variable and what the app hands its sidecar.
- **Its own log says the window opened and the icon went into the menu bar**,
  neither being visible from outside the process. The Log Directory on macOS is
  `~/Library/Logs/Verkstead`, which a runner has a home for; the file is taken
  away first so a line found in it is this run's. No screen to arrange, a macOS
  runner being a logged-in session with a window server of its own.
- **The binary inside answers `ask --help` and `guide` by path**, which is the
  half of this download a *session* gets and the invariant the Sandbox stands on:
  a bundle carrying the window alone would come up, serve, draw its icon, pass
  everything else here, and hand each session it spawned a binary with no `ask`
  in it. Both verbs rather than the exit status, so a binary answering with some
  other command's usage fails here.
- **The bundle's icon resolves**, read off the mounted `Info.plist` and looked
  for in the mounted bundle, then put through `iconutil` — a name pointing at an
  icns the system cannot parse draws the same placeholder as a name pointing at
  nothing.
- **The signature survived the packing** (Set 889 Q3b). The reason it was
  written is gone — the executable was a shell script, whose signature lives in
  an extended attribute a copy can drop, and it is a real Mach-O again. It is
  kept all the same: an Apple-silicon Mac refuses an unsigned binary, and a seal
  broken by the pack or by the image is the "damaged" a Mac will not open rather
  than the "unidentified developer" it offers a way past. Over the mounted
  bundle and over the sidecar inside it.

**What goes**: the `Verkstead-launcher --help` step, there being no launcher;
the `--no-open --listen --data-dir` command line, the app taking none of it; and
the whole Rust build. The unmount stays, `if: always()`, polite and then forced
— the mount is inside the directory the artifact is read out of.

**What is untouched**: `publish`. It fetches desktop artifacts by the
`desktop-*` prefix so they are never counted among the five bare binaries, and
it is where `Verkstead-universal.dmg` is written down. Nothing downstream of
this leg knows the file was made differently.

**Then the script goes**, and with it the hand-written `Info.plist`, its
`LSUIElement`, the launcher, the by-hand `codesign` and the `lipo` over two
cargo builds. `docs/development.md`'s build list loses the line that names a
file which is no longer there; the paragraph describing what the dmg *is* is
task 06's to rewrite.

**What stands as proof** (Set 889 Q2). `release.yml` cannot be rehearsed from
this branch: it is dispatch-only, and `prepare` checks out `main` whatever ref
started the run and pushes a version commit to it, so a dispatch from here would
build main's tree and run main's leg. So the leg is proven by `actionlint` over
the workflow, and by a temporary `macos-15` job added to `ci.yml` under
`pull_request` that runs the leg's steps end to end — building the two headless
CLIs on the runner in place of downloading them, an Apple host cross-compiling
to the other Apple target — kept for this branch and dropped before it merges.
The download step itself is the same two lines `desktop-linux` already runs
against the same matrix.

## Acceptance criteria

- [ ] The leg's steps, run on a Mac against a CLI pair built from this branch,
      pack the dmg and pass every assertion — and each assertion fails, naming
      what went wrong, when broken on purpose.
- [ ] `tools/build-macos-dmg.sh` is gone, nothing in the repository writes
      `Verkstead-launcher` or `LSUIElement`, and `actionlint` and CI are green.
- [ ] `publish` is untouched and still right: `desktop-macos` arrives carrying
      `Verkstead-universal.dmg`, and the count of bare CLI binaries is still
      five.
