# 02. The release leg, and the WiX sources retired

## What to build

`desktop-windows` becomes Electron's leg, and `tools/verkstead.wxs` and
`tools/build-windows-msi.sh` are deleted in the same slice — a leg that no longer
runs the script and a script nothing runs are one change, which is how stages 05
and 06 retired the Rust AppImage's and the Rust dmg's. The UpgradeCode is already
out of the first of those, task 01 having carried it into
`desktop/electron-builder.yml`; **check that before deleting anything**, because
that file is the only record of it.

**It waits on the CLI matrix now**, as the other two desktop legs do, and that is
what makes the sidecar the released artifact rather than a second build of it
(ADR-0020): it downloads `verkstead-windows-x64.exe` — the binary the matrix
already builds and runs — and hands it to `pnpm run pack`. A `needs` cannot name
one row of a matrix, so it names the whole `build` job, where today it names
`viewer`.

**And it compiles nothing.** No Rust toolchain, no `rust-cache` keyed `msi`, no
`touch` of the viewer's embed to defeat cargo's staleness, and no download of the
viewer at all — the viewer is already inside the binary the matrix built. The
runner stays `windows-2025`: candle and light are Windows programs, electron-builder
fetches them itself, and the install and every assertion after it want a real
Windows to happen on. Where the artifact lands moves with the pack, from
`target/windows` to `target/electron/out`.

**The Release's version reaches the package now** (Set 5 Q3). electron-builder
reads the msi's ProductVersion out of `desktop/package.json`, which says `0.1.0`
and which nothing bumps — `prepare` bumps `Cargo.toml`. Left alone, every
Release's msi would read `0.1.0.0`, Installed apps would say `0.1.0` for ever,
and every upgrade would rest entirely on `AllowSameVersionUpgrades`, with a
downgrade indistinguishable from an upgrade. So `prepare` writes the version into
`desktop/package.json` beside `Cargo.toml` and commits both. That commit is one of
the two that reach `main` with no pull request — see `docs/agents/git-workflow.md`,
which is where the exception and its reasoning live — so what it touches is worth
naming there rather than only in the workflow.

**The assertions are on the file that is uploaded**, installed with `msiexec /i
/qn /norestart /l*v` the way a download installs, and everything afterwards asked
of the install it left. Most of them exist already and are reshaped rather than
written:

- **The msi installs**, and a non-zero exit dumps the tail of the verbose log,
  which is UTF-16 and needs the nulls stripped before anything can read it.
  Unchanged.
- **It installed into this user's profile and nowhere else** — the Verkstead
  product registered once to this user and not at all to the machine, and an
  entry somewhere Installed apps can see it. Unchanged, except that the files it
  looks for are the launcher at the root and the CLI under `resources\cli` rather
  than two exes side by side. **Worth watching which hive the uninstall entry
  lands in**: `docs/adoption.md` carries a parked paragraph about the Rust msi
  registering it under `HKLM` where it should have been `HKCU`, and if
  electron-builder's package does not repeat that, task 05 retires the paragraph
  rather than rewording it.
- **The Start-menu entry opens the app, and opens no console with it.** The
  target is `Verkstead.exe` at the install root now rather than the shim, and the
  PE subsystem check stays exactly as it is: what it asserts is that whatever the
  entry opens is a windows-subsystem program, and an Electron launcher is one.
  The shortcut is read through `WScript.Shell`, which reads a target off a
  non-advertised shortcut — electron-builder's template sets
  `DISABLEADVTSHORTCUTS`, so that is what it gets.
- **`verkstead guide` answers in a fresh terminal**, off a `PATH` rebuilt from the
  machine's and the user's stored values rather than the one the step inherited.
  What it now has to resolve to is `resources\cli` inside the install, **and the
  root must not answer**: a lookup that found `Verkstead.exe` would be the promise
  kept backwards, and the assertion is worth making both ways round.
- **The window comes up, the icon goes into the tray, and the viewer is inside
  what was installed.** The app takes no flags — there is no `--no-open`, no
  `--listen` and no `--data-dir` on it — so `VERKSTEAD_DATA_DIR` is how the Data
  Directory is said, which is the server's own variable and what the app hands
  its sidecar. The log lines to wait for are the two the other two legs wait for
  rather than the tray app's one, and the log is under `%LOCALAPPDATA%\Verkstead`.
  Bounded at five minutes like the other two, because the failures this app draws
  are dialogs and a dialog nobody dismisses would hold the runner for six hours.

**And one assertion is new, in two halves** (Set 5 Q2). No leg has ever installed
a package over another, and Windows is the only platform where the question
arises:

- **The msi installed twice over itself**, which is the same-version upgrade rule
  exercised deterministically — and which has to leave one product, one `PATH`
  entry and one Start-menu entry rather than two.
- **And the last Release's msi upgraded over**, which is the half that actually
  tests the carried-over UpgradeCode: the previous package is fetched from the
  latest Release, installed, and this one installed on top of it, with the same
  three counts asserted afterwards. **Where there is no Release to fetch, the
  half is skipped with a notice rather than failing** — a Release should not fail
  for the absence of an earlier one — and the notice has to say plainly that the
  upgrade went unchecked, so a skip cannot be mistaken for a pass.

**What stands as proof.** `release.yml` cannot be rehearsed from this branch: it
is dispatch-only, and `prepare` checks out `main` whatever ref started the run and
pushes a version commit to it, so a dispatch from here would build main's tree and
run main's leg. So the leg is proven by `actionlint` over the workflow, and by a
temporary `windows-2025` job added to `ci.yml` under `pull_request` that runs the
leg's steps end to end — building the headless CLI on the runner in place of
downloading it — kept for this branch and dropped before it merges, the way stage
06's temporary dmg job was. The download step itself is the same two lines the
other two legs already run against the same matrix.

**What is untouched**: `publish`. It fetches desktop artifacts by the `desktop-*`
prefix so they are never counted among the five bare binaries, and it is the one
place `Verkstead-x86_64.msi` is written down. Nothing downstream of this leg knows
the file was made differently. `manifest` is untouched for the same reason — it
hashes the four nix-system binaries and the Windows CLI is not among them.

**Then the two `tools/` files go**, and with them the hand-written WiX source, the
`Cli`/`Shim`/`StartMenu` components, the `cargo build` with the `desktop` feature
left on, and the last unpinned tool in the pipeline. `docs/development.md`'s build
list loses the line naming a file that is no longer there; the paragraph
describing what the msi *is* is task 05's to rewrite.

## Acceptance criteria

- [ ] The leg's steps, run on a Windows runner against a CLI built from this
      branch, pack the msi and pass every assertion — and each one fails, naming
      what went wrong, when broken on purpose.
- [ ] Installing the msi over itself, and over the msi fetched from the latest
      Release, each leaves one product in Installed apps, one `PATH` entry and
      one Start-menu entry — and a run with no Release to fetch says in as many
      words that the second half went unchecked.
- [ ] `tools/verkstead.wxs` and `tools/build-windows-msi.sh` are gone, the
      UpgradeCode is in `desktop/electron-builder.yml`, nothing in the repository
      runs `candle` or `light` by name, and `actionlint` and CI are green.
- [ ] `prepare` writes the Release's version into `desktop/package.json` beside
      `Cargo.toml`, and `publish` is untouched: `desktop-windows` arrives
      carrying `Verkstead-x86_64.msi` and the count of bare CLI binaries is still
      five.

## What was checked here, and what the runner has to check

**Nothing in this task was installed on the machine it was written on, and
deliberately** — for the reason task 01 records at length: this package carries
the live Verkstead's `UpgradeCode` by decision, and the Verkstead running the
session is an install of that same product, so `msiexec /i` here is a major
upgrade of the orchestrator doing the testing rather than a test of the package.
The upgrade assertion this task adds is the same act three more times over, and
its second half uninstalls first. So the whole of it belongs on a runner.

What stands in for it here is what the task said would: `actionlint` over both
workflows, which is green, and `shellcheck` over every `run:` script in them,
which reports nothing on any script this change wrote. The three findings it does
report are the ones `main` already had — two `ls | grep` in the dmg leg and an
`ls` in `publish` — and the four `tr 'A-Z' 'a-z'` infos it used to report in this
leg are gone with the lines they were on, the new comparisons using
`[:upper:]`/`[:lower:]`.

**The two were run separately rather than as one command, and not by choice.**
`actionlint -shellcheck` deadlocks on this machine for any `run:` script over
about four kilobytes — reproduced against a synthetic workflow whose only script
is a hundred `echo` lines, and against `main`'s own `release.yml` with one long
step appended, so it is nothing about what is written here. actionlint's own
checks pass with the integration off, and `shellcheck` was run over each script
extracted from the block scalars instead, which is the same analysis in two
commands.

The counting the upgrade assertion rests on, and the `PATH` comparison the guide
assertion rests on, were each exercised here against made-up inputs: one row,
two rows, none; the CLI's directory found, the root on `PATH`, nothing found, and
some other `verkstead` found first. Each says which claim failed and why. And
`prepare`'s bump was run against a copy of `desktop/package.json` with
`VERSION=0.1.3-rc.2`, which leaves that file one line different and reads `0.1.3`
back out of it.

**What the temporary `the-windows-msi-leg` job in `ci.yml` is there to answer**,
and what nothing read off this branch can: that the leg's steps pack an msi
Windows really installs, that each assertion holds against the install it left,
and that each fails naming what went wrong when broken on purpose. It builds the
headless CLI on the runner in place of downloading it from the matrix, because a
`build` job is the one thing that cannot come along to a pull request; every step
after the pack is the leg's word for word. It goes before the branch merges, the
way stage 06's temporary dmg job did.
