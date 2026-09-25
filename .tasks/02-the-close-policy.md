# 02. The close policy, over the app's own settings

## What to build

Closing the window stops being a quit and becomes a choice, and the choice is
kept in a JSON file of the app's own under Electron's user data — beside the file
the window's size and position are already remembered in, and for the same
reason: this is a fact about the desk in front of the human rather than anything
the server was told. Nothing about it is on the wire and nothing is in
`config.yaml` (ADR-0020).

**Two settings, and the shape is worth fixing here** because tasks 03 and 04 read
it:

    { "whenClosed": "tray" | "ask" | "quit", "trayIcon": boolean }

with `"tray"` and `true` the defaults. A file that is missing, unreadable, or
holding something else reads as the defaults **one setting at a time** — the
reading the remembered bounds and the viewer's own per-device storage both make,
so a hand-edited file with one bad line still carries the other.

**The three positions.** Keep running in the tray hides the window and leaves the
app and the sidecar up. Ask before quitting raises a native dialog saying running
sessions will be stopped, with **Quit** and **Cancel**. Quit quits. What the app
does today — every closed window quits — is what this replaces.

**With the tray off, the choice falls to Quit** whatever the file says, and not
to "ask first": a hidden app with no icon is a Verkstead nobody can reach
(Set 846 Q7a). The fall-through is enacted here; the greyed radio position and
the note saying why are drawn in task 04.

**The warning is the close button's alone** (Q7b). The tray's **Quit** and the
menu's Quit shortcut quit at once, because somebody who chose Quit has already
said what they meant.

**A close that hides must still remember where the window was.** The bounds are
written on the window's own close, and a close that is cancelled in favour of
hiding has to leave that write standing — otherwise an app that always keeps
running is an app that never remembers its window again.

**A Mac is the platform's own**, and the arm is written here and proven in stage
06 (Q3b, settled with the human): closing leaves the app in the Dock, a Dock
activation brings the window back, and the radio is not consulted at all there.
Cmd+Q quits, as it does everywhere.

**What vitest covers**: reading and writing the settings file — the defaults, a
partial file, a corrupt one, and a round trip — and the policy as a function of
the position, the tray setting and the platform, which is where the fall-through
and the Mac arm are decided. Enacting it is the window's, at the edge.

## Acceptance criteria

- [ ] Each of the three positions does what it says with the setting hand-edited
      into the file between runs, and a close that hides still leaves the
      window's place remembered for the next run.
- [ ] The tray off with keep-running chosen quits on close, and the warning is
      never raised by anything but the close button.
- [ ] On a Mac the close leaves the app running with no radio consulted and the
      window comes back — written here as the arm stage 06 proves.
