# 05. The words

## What to build

The Windows half of the documentation, which still describes the Rust tray app
throughout. Three documents and one ADR, and what each of them owes:

**`docs/adoption.md` is the biggest of the four**, because its Windows section was
never about an Electron app at all. Linux's and the Mac's have both been rewritten
by the stages before this one and the shape they landed on is the shape to follow:
name the file and what is in it, then the platform's refusal and the numbered way
past it, then where it installs and how it starts, then the **window has no title
bar** paragraph, then the logged-in window, the phone, what is inside, and
sessions. What is wrong there now:

- **It describes an icon that opens a browser.** The notification-area menu is
  **Open**, **View Logs** and **Quit**, and what a press of the Start-menu entry
  gets is a window of Verkstead's own. Launch on Startup is not on that menu any
  more: it is on the **Desktop** section of the settings page, with **When the
  window is closed** and **Show tray icon** beside it.
- **There is no no-title-bar paragraph**, where Linux and the Mac each have one.
  Windows gets the controls overlay at the top-right, drawn on the paper the page's
  heads are drawn on, with the double-click and `Ctrl+M` gestures beside it — and
  whatever task 03 found out about it on a real machine.
- **The `PATH` sentence promises the wrong directory.** It says "the install
  directory goes on your `PATH`", which was true of two exes side by side and is
  not true of this install: what goes on it is the CLI's own directory inside the
  install, because the root holds a launcher named for the product and Windows
  resolves a lookup without regard to case. The promise the sentence is making —
  that `verkstead ask` and `verkstead guide` work in a terminal opened after the
  install — is unchanged and is the thing to keep saying.
- **The two files become one plus a launcher.** Nothing a downloader gets is a
  shim any more and nothing has a `desktop` verb; the SmartScreen steps, the
  per-user install, the **Installed apps** row and the "a newer msi replaces the
  copy that is there" promise all stay exactly as they are.
- **And the parked paragraph about the uninstall entry in the wrong hive** is task
  02's finding to act on: retired if electron-builder's package registers it under
  `HKCU`, reworded if it does the same thing the Rust msi did.

**`docs/releasing.md`** owes its Windows-leg paragraph, which currently names both
retired `tools/` files and describes a leg that compiles Rust. It becomes the leg
as task 02 left it — the CLI downloaded from the matrix, electron-builder's WiX
target, the CLI's own directory on `PATH` — and the sentence about the WiX toolset
being the runner image's own goes, electron-builder fetching its own. The
three-numbers paragraph wants its arithmetic corrected rather than deleted: the
version is four numbers now and only the first three are compared, which is still
why the package allows an upgrade from a version reading the same as its own. The
list of what each desktop leg asserts gains the upgrade, and the note that the
Windows leg checks two of the installer's own gains the third. And the version
commit now touches `desktop/package.json` as well, which is worth saying where the
commit is described.

**`docs/development.md`** owes two things. Its build list drops
`tools/build-windows-msi.sh`, and the paragraph under the list loses the sentence
saying the last entry is the Windows desktop artifact — all three are the packed
Electron app now. Its msi paragraph becomes what the AppImage's and the dmg's
beside it are: "the same pack on Windows", the console block a developer actually
runs, what comes out and where, which directory is on `PATH`, and the platform
restriction at the end. The paragraph saying the Rust tray app is still what a
release carries on each platform until that platform's stage goes with it —
Windows was the last one — while `crates/desktop` itself stays, being stage 08's.
And the `.ico` paragraph loses its mention of `tools/verkstead.wxs`.

**`docs/adr/0020-electron-desktop.md`** owes one sentence. Its packaging decision
says the msi comes "through electron-builder's WiX target with a fragment of our
own putting the CLI on the user's PATH", and electron-builder's msi target has
nowhere to put a fragment — the template is its own, candle gets one source and
light one object. What the decision actually rests on is the `msiProjectCreated`
hook, and the ADR should say so: the decision is unchanged and the mechanism named
in it was wrong. Amended in place, the way ADR-0012 was.

**And `CONTEXT.md`** is worth a read rather than an assumption. **Startup
Registration** already says the Run key on Windows and already describes the Mac's
launch agent being taken over once; whether the Windows value's own take-over
belongs beside it, and whether **Window Decorations** needs anything after task
03's looking, is this task's to judge. Terms are updated as a piece lands, which is
what this is.

**What nothing here does is touch the crate.** `crates/desktop` and its shim are
stage 08's to remove, and this task leaves them alone — so the criterion is about
what the documentation offers a downloader, not about what is still in the tree.

## Acceptance criteria

- [ ] Adoption's Windows section describes the Electron app throughout — the
      window with no title bar and the overlay at its corner, the tray menu's three
      items, Launch on Startup on the Desktop section, and the directory that is
      actually on `PATH` — and its SmartScreen steps, per-user install and
      replace-rather-than-stand-beside promise are intact.
- [ ] Development's build list has no `tools/build-windows-msi.sh` in it and its
      msi paragraph reads like the AppImage's and the dmg's; releasing's Windows
      leg paragraph describes the leg that is there, with the upgrade among the
      assertions it lists.
- [ ] ADR-0020's packaging sentence names the mechanism the `PATH` entry actually
      uses, and nothing in the documentation offers a downloader the shim, the
      `desktop` verb, `candle` or `light`.
