# 01. The msi, packed from a checkout

## What to build

`desktop/electron-builder.yml` gains the `win:` section it was written to have
room for, beside the `linux:` and `mac:` ones, and the `msi:` section under it.
`pnpm run pack` on Windows then produces `Verkstead-x86_64.msi` — the Electron
app with the headless `verkstead.exe` packed inside it as its sidecar — and that
file installs into the profile, puts an entry in the Start menu, puts the CLI's
own directory on the user's `PATH`, and leaves one Verkstead where there was a
Rust one before it.

**First, because nothing after it can be looked at.** `startup.ts` answers
`nowhere` on an unpackaged run, by decision (ADR-0020, Set 847 Q2): a
registration made from a checkout would name the dev shell's Electron in the nix
store. So Launch on Startup, the Run value take-over and a hidden sign-in start
are all unreachable from `pnpm start`, and the overlay is worth proving in the
window a downloader actually gets. This is the same reason stage 06 packed its
dmg before it looked at anything else.

**The launcher is `Verkstead.exe` and the CLI is a directory deeper, which is
what the `PATH` entry has to name.** electron-builder names the launcher for the
product, so the install root holds `Verkstead.exe`; stage 05's extra resource
puts the sidecar at `resources\cli\verkstead.exe` inside the install. Windows
resolves a `PATH` lookup without regard to case, so a root on `PATH` would answer
`verkstead guide` with the window instead of the Guide — and a rename cannot fix
it either, one NTFS directory not holding `verkstead.exe` and `Verkstead.exe`
both. So `resources\cli` is what goes on `PATH`, and the root goes on it no more.
That layout is already what `src/cli.ts` looks in and what `electron-builder.yml`
already stages, so nothing about the app moves for this.

**And the `PATH` entry is a patch of the generated project rather than a fragment
beside it.** This is the one place the brief and ADR-0020 describe a mechanism
electron-builder has not got: on the pinned version its msi target reads its
`template.xml` out of `app-builder-lib` with no `buildResources` override, hands
candle one `project.wxs` and light one `project.wixobj`, and there is nothing a
second WiX source could be linked into. What it does have is **`msiProjectCreated`**,
a configuration key naming a JS file that runs after the wxs is written and
before candle compiles it. So that hook is where the `PATH` entry comes from, and
where the install directory's name is forced. Two things follow:

- **It is string surgery on a generated file**, so it has to fail loudly rather
  than silently: a patch that found nothing to change must stop the pack rather
  than quietly produce an msi with no `PATH` entry in it. That is the one way
  this breaks on an electron-builder upgrade, and the assertions in task 02 are
  the other half of the answer.
- **candle runs `-pedantic -wx` and light runs its ICE validation** on Windows,
  so whatever the hook writes has to pass both. `msi.additionalWixArgs` and
  `msi.additionalLightArgs` are the escape hatches if it does not; reaching for
  them is worth a note saying which check was in the way.

The entry itself is what the retired `tools/verkstead.wxs` wrote, retargeted:
`Name="PATH"`, `Action="set"`, `Part="last"` so it appends to what the user's
`PATH` already said, `System="no"` so it lands in `HKCU` where a per-user install
belongs, and `Permanent="no"` so the uninstall takes it away with the files.

**What goes in the configuration, and what is already there.** Two of the
brief's worries turn out to be free, and the third is a plain option:

- `msi.upgradeCode` is a configuration key, so the carried-over
  `{A4727089-F5B9-40A6-A196-1856E8D6827A}` goes here. **This is the task that
  carries it out of `tools/verkstead.wxs`, and that file is the only record of it
  until this lands** — task 02 deletes the file. It is the one identifier that
  says two packages are the same product; derived afresh from the app id, an
  upgrade would install beside the Rust msi rather than over it.
- `AllowSameVersionUpgrades="yes"` and light's `-sw1076` are already in
  electron-builder's own template, so the same-version upgrade rule
  ([releasing.md]) needs nothing added.
- Nothing has to install WiX. electron-builder downloads its own `candle` and
  `light` and caches them, which is one fewer unpinned tool than the Rust leg
  had.

**And four things about the package are decided rather than defaulted** (Set 5
Q4):

- **The install directory is `Verkstead`.** electron-builder derives it from
  `package.json`'s `name`, which would give `%LOCALAPPDATA%\Programs\verkstead-desktop`;
  the same hook forces it to the `…\Programs\Verkstead` the Rust msi used and the
  adoption documents name.
- **The publisher is `Tobias Cohen`**, which electron-builder reads from `author`
  in `desktop/package.json` and which that file has not got. It has to be the
  object form — `"author": { "name": "Tobias Cohen" }` — because what is read is
  `author.name`, and a bare string leaves it undefined and falls back to the
  product name. Adding it also gives the Mac bundle a copyright string it has not
  had, which is worth looking at rather than being surprised by.
- **The installer starts the app when it finishes**, which is electron-builder's
  default and not what the Rust msi did. A silent install is unaffected — the
  custom action is conditioned on the UI level — so this shows up for a human
  double-clicking the file and nowhere in CI.
- **No desktop shortcut.** The Start-menu entry is what this stage promises, and
  electron-builder creates a desktop one unless told not to.

**The artifact's name is written down rather than defaulted**, for the reason the
AppImage's and the dmg's are: `publish` in `release.yml` checks the desktop
assets by name and `Verkstead-x86_64.msi` is one of the three it names, the
adoption documents spell it, and electron-builder's own pattern would put the
version in the middle of both.

**The version is four numbers now, and only the first three are compared.**
electron-builder writes `major.minor.patch.0` out of `desktop/package.json`,
where the Rust script truncated a tag's pre-release suffix to three. Nothing
about that is wrong — Windows Installer ignores the fourth field when it
compares — but that file says `0.1.0` and nothing bumps it; making the Release's
version reach the package is task 02's, along with the leg.

## Acceptance criteria

- [ ] An msi packed from a checkout on Windows installs under
      `%LOCALAPPDATA%\Programs\Verkstead` without asking for administrator, and
      the app it leaves opens its window, puts an icon in the notification area
      and serves the workbench — its own log under `%LOCALAPPDATA%\Verkstead`
      saying both.
- [ ] `verkstead guide` in a terminal opened after the install prints the Guide
      rather than opening a window, and `where verkstead` answers with the file
      under `resources\cli` rather than the launcher at the root.
- [ ] Installed over an msi built from `main`'s `tools/build-windows-msi.sh`,
      it leaves one row in Installed apps, one `PATH` entry and one Start-menu
      entry — and the uninstall takes the `PATH` entry away with the files.
- [ ] `pnpm run pack` still takes one path and packs an AppImage on Linux
      unchanged, and two paths and a dmg on a Mac.

[releasing.md]: ../docs/releasing.md

## What was checked, and what a disposable machine still has to check

**The install half of the three criteria above was not run on the machine this
was built on, and deliberately.** This package carries the live Verkstead's
`UpgradeCode` by decision — that is the whole point of carrying it — and the
Verkstead running the session that builds it is an install of that same product,
registered machine-wide. So `msiexec /i` here is not a test of the package but a
major upgrade of the orchestrator doing the testing: `FindRelatedProducts`
matches, `RemoveExistingProducts` takes the running `verkstead.exe` and
`verkstead-desktop.exe` out from under themselves, and the `PATH` and Start-menu
entries go with them. Two earlier attempts at this task did exactly that and
stopped Verkstead both times — the second only after the downgrade block
(`WIX_DOWNGRADE_DETECTED`, a `1603` reading *a newer version of Verkstead is
already installed*) had been worked around by bumping the version past the live
one, which is the block doing its job.

So what stands in for the install here is the package's own account of itself,
read out of its tables: the carried-over `UpgradeCode` and the upgrade row that
searches on it, `VersionMax` inclusive for the same-version rule,
`ALLUSERS=2` with `MSIINSTALLPERUSER=1`, `APPLICATIONFOLDER` named `Verkstead`
under `ProgramFiles64Folder`, one `Environment` row appending
`[APPLICATIONFOLDER]resources\cli` and removed on uninstall, that row's
component being the one holding `verkstead.exe`, the launcher at the root, a
Start-menu shortcut and no desktop one. `scripts/msi.mjs` was exercised
separately against a project of the generated shape and against four manglings
of it, every one of which stops the pack rather than writing an msi with no
`PATH` entry in it.

**What still wants a machine nobody minds losing**, because nothing read off the
package can answer it: that the app the install leaves opens its window, reaches
the notification area and serves the workbench; that `verkstead guide` in a
terminal opened afterwards prints the Guide and `where verkstead` answers with
the file under `resources\cli`; and that installing over an msi built from
`main`'s `tools/build-windows-msi.sh` leaves one row in **Installed apps**, one
`PATH` entry and one Start-menu entry. The uninstall's half of the third — that
the `PATH` entry goes with the files — did get seen here, in the course of
clearing an earlier attempt's install away: the entry was in this account's
`HKCU` `Path` before `msiexec /x` and gone after it.
