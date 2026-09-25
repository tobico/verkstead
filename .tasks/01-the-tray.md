# 01. The tray

## What to build

An icon in the system tray, with **Open**, **View Logs** and **Quit** on its
menu, and a click on the icon that opens the window (ADR-0020). The tray is
shown unconditionally here — **Show tray icon** is a setting that arrives in
task 03, and a switch with nothing behind it is worse than a tray that is
honestly always on.

**Open is the window put back in front of the human**, whether it is minimised,
buried or — from task 02 — not on the screen at all. The app already has the
three-step bring-forward that a second launch of the app uses; this is the
second caller of it.

**View Logs opens this run's log file** with whatever the platform reads text
with, and says so instead where there is none. The app's logging already answers
that question for itself: opening the log file hands back either the file it
opened or a human-worded reason there is no file and the lines are going to
standard error, and the entry file presently drops that answer. Hold it, and let
the menu item open the one or show the other. A machine with nowhere to keep a
log file has only lost the log, so the item says that rather than failing
silently or opening nothing.

**Quit never asks.** Somebody who chose Quit has said what they meant, and the
warning that arrives in task 02 is the close button's alone. Quitting stops the
sidecar, which the app's own quit handling already does.

**The icon is the packaging artwork, from the repository while the app is
unpackaged.** The Rust tray app draws the 192px PNG of the generated icon set —
more than any panel asks for, and a panel scales down rather than up. Every run
in this stage is `pnpm start` from a checkout, so the path resolves out of the
source tree; where a packed app finds it is stage 05's, and this should be one
question asked in one place rather than a path written twice.

**Two things about Linux are already known**, so neither is worth rediscovering:

- The pinned Electron constructs a tray with no StatusNotifierWatcher on the
  session bus, takes a context menu and is not destroyed. So a desktop with no
  tray host needs no guard, and nothing here has to survive a throw.
- A click is the panel's own choice of gesture. Electron emits its click on the
  StatusNotifierItem activation, but the specification does not say what causes
  one — some panels send it on a left click, some on a double click, and some
  only open the menu. So the click is wired, **and Open is the first item on the
  menu**, which is what the Rust tray relied on for the same reason. What COSMIC
  actually sends is task 06's to record.

**What vitest covers**: the menu as a value — which items there are, in which
order, and what each one means — and the reading of where the logging went into
what the item does about it. The `Tray` itself is a thing rather than a value,
so it joins the short list of files allowed to import `electron`, with a line
saying why it is on it.

## Acceptance criteria

- [ ] A minimised or buried window comes back on **Open**, and on a click where
      the desktop sends one.
- [ ] **View Logs** opens this run's log file, and a run with nowhere to keep one
      says where its logging went instead of opening nothing.
- [ ] **Quit** ends the app and takes the sidecar with it, with no warning of any
      kind.
