# 07. Windows

## Goal

`Verkstead-x86_64.msi` on a Release installs the Electron app per user under
`%LOCALAPPDATA%\Programs`, with a Start-menu entry and the CLI's own directory
inside the install on the user's PATH, so `verkstead guide` works from a fresh
terminal and starts the CLI rather than the app. The controls overlay sits at
the top-right in the heads' colours; Launch on Startup is the Run key through
the login-item API, taking over the tray app's value where there is one; the
`desktop-windows` leg installs the msi and asserts the install, the record
under HKCU, the Start-menu entry, the PATH, the upgrade over the package
before it, and the three assertions every leg makes. The WiX sources for the
Rust msi are gone, the UpgradeCode carried out of them first.

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
5. **The words** — the Windows sections of adoption, releasing and development
   rewritten, development's build list dropping `tools/build-windows-msi.sh`
   and its msi paragraph describing what the install now is and which directory
   is on `PATH`. Accepts: nothing names the shim.

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

## What task 03 found on Windows

The run this stage owed on the overlay: the packed app on Windows 11 (26200) at
200% display scaling under the pinned Electron, with the real `verkstead`
sidecar serving the real workbench, driven by the DevTools protocol for what the
page measures and by Win32 for what the window does. **Two things were wrong,
both are fixed here, and the rest of stage 04 holds exactly as written.**

**The overlay is the head, and the band is the page's.** `titleBarStyle:
"hidden"` gives a window with no title bar and the three controls drawn over the
top-right corner of the page. `getTitlebarAreaRect()` answers
`{ x: 0, width: innerWidth - 137, height: 75 }` — the height being the band the
page pushed and the app logged, 75px at a sixteen-pixel root — and the strip the
controls took is 137 CSS pixels, three buttons of about 46 each. Photographed off
the window: that strip is painted `#faf8f5` with its symbols in `#1c1a17`, which
are the head's `--paper` and `--ink` to the byte. A flip of the scheme recoloured
it in the same run — `#171614` paper with `#ece7e0` marks, pushed, logged and
repainted with no restart — and flipping back put it as it was.

**And 137 is 137 at either scaling.** The same window under
`--force-device-scale-factor=1` reports the same 137 CSS pixels taken and the
same 75-pixel band: the platform's furniture is drawn in the display's own
pixels, so what the page is left is the same number of the page's own, and the
band is a sum in rem that scaling never enters. Which is the answer to the
question the stage raised about a machine that is not at 100% — there was
nothing there to answer for.

**Snap layouts are offered, and the heads move the window.** Hit-testing the top
band a point at a time is what the platform asks the window, and the whole of
what it decides either from: across a 2400-pixel window the answers run `CAPTION`
from 56, a `CLIENT` island where the head's own controls stand, `CAPTION` again
to 2115, then `MINBUTTON`, **`MAXBUTTON`** and `CLOSE` in three even blocks to
the frame. `HTMAXBUTTON` is exactly what Windows wants back before it offers snap
layouts on a hover, so the reason ADR-0020 kept the platform's controls is
discharged. The `CAPTION` stretches are the pane heads: a `WM_NCLBUTTONDBLCLK` on
one maximised the window and a second one restored it, and `Ctrl+M` through the
hidden menu bar's **Window** minimised it. No page control hit-tests anywhere
inside the three buttons.

### The inset was read against the wrong window

**`geometrychange` fires before the page's `innerWidth` has caught up.** The
inset is a sum of two readings — the window's width, less the strip the page was
left — and `controls.ts` recomputed it on the overlay's event alone. Windows
delivers the two a moment apart, which a listener on each proved: dragging a
window from 1187 to 787 CSS pixels across fired `geometrychange` carrying the new
rectangle, 650 wide, against the *old* `innerWidth` of 1187, and the `resize`
that followed carried 787. So every reading after the first paired a new
rectangle with a stale window:

- shrinking the window padded the head by the difference between the two —
  `--controls-right: 737px` on a window 487 across, the head's content driven off
  the side and the sidebar's head collapsed to nothing;
- growing it computed a *negative* inset, which `insets` floors at nought, so the
  frame carried no variable at all and the three controls stood over the head the
  padding exists to keep clear;
- and a maximise, which moves the width by a single pixel here, was wrong by one
  — which is how it was noticed.

Only the reading at load was ever right, both halves being settled by then.
COSMIC delivered the two together and never showed it (stage 04), and the unit
tests moved the rectangle with `innerWidth` held still, so neither had a way to.
**The fix is to follow `resize` as well** — the event carrying the half the
overlay's own does not — with a test that moves both. Re-measured on the same
machine afterwards: at 487, 687, 887, 987, 1187, 1267 and 1280 CSS pixels across,
through a maximise and a restore, the stored inset is 137 and the live one is
137, and nothing of the page's is under the controls at one pane or at two.

### The window was not the Start-menu entry's

**Nothing said what this application is called, so the taskbar could not tie the
two together.** The msi's Start-menu shortcut carries
`System.AppUserModel.ID = net.tobico.Verkstead` — read out of the built package's
`MsiShortcutProperty` table — and the packed app set no id at all: the window
carried none of its own, and the process answered to Electron's derived
`electron.app.Verkstead`. Windows hands a shortcut's id to what it starts, so a
launch *through* the entry would have grouped; every other way in would not have.
The installer's own run-after-finish, the Run key at login (task 04's) and the
launcher double-clicked in the install directory would each have drawn a second,
unnamed button beside the entry. **The fix is `app.setAppUserModelId(APP_ID)`** —
the same string the registrations are already named for, and the same one
`electron-builder.yml` packs as the `appId`. It is the Windows arm of what
`syncDesktopName` does on Linux. The rebuilt app reports `net.tobico.Verkstead`,
which is the shortcut's own.

### What was not reached, and why

- **The three-pane frame was drawn, but its far head was not.** This display is
  1280 CSS pixels wide, which is exactly the 80rem breakpoint, and the only page
  that hands the frame a middle pane without work having been run is **Settings**
  — whose details pane draws no head at all. So all three columns stood, the
  sidebar's head was padded by nothing, and the overlay sat over an empty column
  with nothing under it; but the rule that pads a *third* column's head is the
  stylesheet's, and is covered where it always was, in `decorations.test.tsx`.
- **The app was not installed from the msi.** This machine's Verkstead is the
  Rust tray app, running out of `%LOCALAPPDATA%\Programs\Verkstead` — the
  directory the msi installs into, under the UpgradeCode it carries — so
  installing it here would have taken the running server down with it. The window
  was proven from the packed app run out of a copy of `win-unpacked`, told to
  serve on a port of its own and handed a Data Directory of its own, so that
  nothing it did could reach the machine's Verkstead. What the install does to a
  profile is the `desktop-windows` leg's, and that leg asserts it.
- **`desktop`'s own unit tests do not pass on Windows**, and did not before this
  task either: 40 of 269 assert POSIX path shapes — XDG directories, a Mac's
  `~/Library`, an autostart entry's path — against a `join` that answers in
  backslashes. They are the Linux runner's, CI runs them there, and nothing here
  moved the count.

[ADR-0020]: ../../adr/0020-electron-desktop.md
[releasing.md]: ../../releasing.md
