# 07. Windows

## Goal

`Verkstead-x86_64.msi` on a Release installs the Electron app per user under
`%LOCALAPPDATA%\Programs`, with a Start-menu entry and the CLI's own directory
inside the install on the user's PATH, so `verkstead guide` works from a fresh
terminal and starts the CLI rather than the app. The
controls overlay sits at the top-right in the heads' colours; Launch on
Startup is the Run key through the login-item API; the `desktop-windows` leg
installs the msi and asserts the install, the record under HKCU, the
Start-menu entry, the PATH, and the three assertions every leg makes. The
WiX sources for the Rust msi are gone.

## Decisions in force

- **An msi, through electron-builder's WiX target** ([ADR-0020], Set 848
  Q16): the human kept the msi over the NSIS installer recommended.
- **A directory with the CLI in it stays on the user's PATH** (Set 850 Q20): a
  WiX fragment of our own in the build, because the target does not do it
  alone. The adoption docs promise it and the leg asserts it.
- **And it is not the install root**, which is what the promise used to mean.
  electron-builder names the launcher for the product, so the root holds
  `Verkstead.exe` — and Windows resolves a `PATH` lookup without regard to
  case, so a root on `PATH` would make `verkstead guide` start the app. The
  CLI cannot join it under its own name either: one NTFS directory does not
  hold `verkstead.exe` and `Verkstead.exe` both. So the CLI gets a directory
  of its own inside the install, that directory is what the fragment names,
  and the root goes on `PATH` no more.
- **Per user, unsigned**, as before: the SmartScreen steps in the adoption
  docs kept.
- **The overlay and its colours** are stage 04's code, proven here.
- **Launch on Startup through the login-item API** (Set 846 Q9a), which is
  the Run key underneath; hidden start when the tray is shown.
- **The tray app's own Run value is taken over, once.** It is named for the
  app id — `net.tobico.Verkstead` — and holds `verkstead.exe desktop`, a verb
  stage 08 removes; Electron's API writes a value of its own and knows nothing
  about that one. So the first launch after an upgrade reads the old value,
  carries whether it was set into the new registration, and deletes it. Read
  once at launch rather than offered as a control: it is a registration this
  app made under another name, not a second setting.
- **Sessions on ConPTY and the AppContainer are untouched**: the CLI inside
  is the same, and the named pipe a container asks through is the server's.
- **The Windows Installer version stays three numbers**, so the package must
  still allow an upgrade from a version reading the same as its own
  ([releasing.md]).
- **And the UpgradeCode is carried over**, not regenerated:
  `{A4727089-F5B9-40A6-A196-1856E8D6827A}`, out of `tools/verkstead.wxs`
  before this stage retires it. It is the one identifier that says two
  packages are the same product, and electron-builder's target derives its own
  from the app id unless it is told. Derived afresh, an upgrade would install
  beside the Rust msi rather than over it — two install directories, two PATH
  entries, two Start-menu entries and two rows in **Installed apps** — against
  what `docs/adoption.md` promises in as many words: a newer msi replaces the
  copy that is there rather than standing beside it.

## Proposed tasks (provisional)

1. **The overlay on Windows** — proven with the stage-04 code; snap layouts
   work. Accepts: no control under the overlay in any pane count.
2. **Login item** — the Run key arm through the API, hidden start, and the
   one-shot take-over of the tray app's `net.tobico.Verkstead` value.
   Accepts: the value appears and disappears with the box and names the app's
   own path; a profile carrying the tray app's value comes up registered
   through the API with that value gone, and one carrying none is untouched.
3. **The msi** — electron-builder's WiX target, per-user, the CLI in a
   directory of its own, the PATH fragment naming that directory, the
   Start-menu entry, the carried-over UpgradeCode, the same-version upgrade
   rule. Accepts: a local build installs under the profile; `verkstead guide`
   runs from a new terminal and prints the guide rather than opening a window;
   installing it over a Rust msi leaves one product, one PATH entry and one
   Start-menu entry.
4. **The leg** — `desktop-windows` downloads `verkstead-windows-x64.exe`,
   packs, installs, and asserts, the upgrade over the last Release's msi
   among the assertions; `tools/verkstead.wxs` and
   `tools/build-windows-msi.sh` retired, the UpgradeCode taken out of the
   first before it goes. Accepts: the leg is green.
5. **The words** — the Windows sections of adoption and releasing rewritten.
   Accepts: nothing names the shim.

## Re-verify at start

- Stage 05 landed; whether 06 has, which decides nothing here.
- electron-builder's msi target on the pinned version, and how a WiX fragment
  is attached to it.
- What the launcher exe ends up called, and where stage 05's extra resource
  puts the CLI inside the packed app — which together decide what the
  fragment names and whether a rename would do instead.
- What `docs/adoption.md` says the `PATH` entry is, so the sentence rewritten
  in task 5 describes the directory that is actually on it.
- The WiX toolset the runner image carries, against what the target wants.
- Whether electron-builder's target takes an UpgradeCode as configuration or
  wants the fragment to carry it, and what `tools/verkstead.wxs` still says
  the code is — it is the only record of it once that file is gone.
- The shim is still in `crates/desktop` and is stage 08's to remove; this
  stage leaves the crate alone.
- What `crates/desktop/src/startup/run_key.rs` writes today — the value's name
  and what it holds — which is what the take-over has to find, and which
  stage 08 deletes the only other reader of.

[ADR-0020]: ../../adr/0020-electron-desktop.md
[releasing.md]: ../../releasing.md
