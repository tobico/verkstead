# 01. The desktop crate boots on macOS

## What to build

`crates/desktop` is GTK all the way down and does not compile for an Apple
target at all: the toolkit is an unconditional dependency, the run loop the main
thread is given is GTK's, and both dialogs are GTK's message dialog. This is the
macOS half of each of those three, with the Linux ones left exactly where they
are.

Run on a Mac, `verkstead-desktop` puts an icon in the menu bar, opens the viewer
in the default browser, and writes its log where stage 01 said a Mac keeps one.
The menu is the same four items it is on Linux — Open, View Logs, Launch on
Startup, Exit — drawn from the same `Chosen` list, so the two platforms cannot
drift into two different menus. Launch on Startup is greyed until the task after
this one gives it a registration to stand for, which is a state the crate
already draws for a machine with nowhere to keep one.

Three things decide the shape:

- **The loop has to be the platform's own, on the main thread.** `Desktop::run`
  blocks the main thread on GTK's loop today and brings the server's ending back
  to that thread to end it. macOS needs the same arrangement over an
  `NSApplication` run loop instead, including the two ways it ends — Exit chosen
  off the menu, and the server stopping underneath it.
- **The dialogs are drawn on the loop's thread**, which is the constraint
  `dialog`'s own documentation spells out: a menu item's handler runs on the
  loop's thread and a dialog raised from it must not need a second one. The
  macOS arm has the same obligation and the same two callers — `main`, before
  there is a tray, and a menu handler.
- **The app is a menu-bar app rather than a windowed one.** There is no window
  behind the icon and there should be no Dock tile either.

Nothing about the tray's menu, its ids, its icon or the double-click binding
changes: `tray-icon` reports a double-click where the platform has one, and on
macOS an icon with a menu attached opens the menu on click — which is the
platform's way and what the stage decided, so the existing binding is already
right and needs no arm of its own.

## Acceptance criteria

- [ ] `cargo build --target aarch64-apple-darwin --package verkstead-desktop`
      and the same for `x86_64-apple-darwin` both succeed
- [ ] On a Mac the icon appears in the menu bar, Open puts the viewer back in
      front, View Logs opens the file, and Exit stops the server — and the log
      carries the line the release leg reads to know a tray came up
- [ ] The Linux tray, its dialogs and its tests are untouched, and `ci.yml`
      stays green
