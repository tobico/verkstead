# 04. The Desktop page

## What to build

A new **Desktop** section at the top of the settings: a card in the middle pane
above the git card, a word added to the settings' own list of word-named openings
so the route and its test arrive with it, and a details pane holding **When the
window is closed** — one radio of three positions, keep running in the tray by
default — **Show tray icon**, and a **View Logs** button. Everything it draws is
read from and written to the bridge task 03 exposed.

**Drawn only where the bridge is.** Without one the settings page is exactly what
it was: no card, no pane, nothing in the middle pane's order changed. That is what
keeps a phone on the tailnet from ever seeing it, and the reason the section is
the app's rather than the server's.

**The greyed tray position.** With **Show tray icon** off, the keep-running
position of the radio is greyed and a note says why — the choice falls to Quit,
because an app with no icon cannot be reached. Greyed rather than taken away: a
position that vanished would say the setting had, and it has not. The page has a
pattern for exactly this, the nested group that is disabled while the box above it
is off, and the browser's own disabling is what greys it and takes it out of the
tab order.

**View Logs is drawn whatever the tray setting says, and on every platform**
(ADR-0020). A switch somebody can turn off cannot be the only way to a log file,
and the desktop most likely to have the tray off is the one the tray misbehaved on
— which is the machine whose log is worth reading. It opens what the tray item
opens, so the two are one action reached two ways.

**The platform arms are written here and proven in 06 and 07** (Q3a, settled with
the human). On a Mac the close radio is not drawn at all — the Dock decides what
closing means there — and **Show tray icon** reads as the menu bar icon. The
platform comes off the bridge, so a stub bridge can claim any of the three.

**Launch on Startup is task 05's.** It arrives with the registration behind it, so
nothing here draws a control that does nothing.

**The page's own conventions hold.** The settings page is a form, so these are the
page's checkbox rather than the sliding switch that sits on a pane head; a control
shows where things stand rather than where somebody pressed, so a press is handed
up as the state it would become and what moves the control is the answer coming
back.

**What the viewer's suite covers**: the section driven through a stub bridge
beside the stubbed fetch the settings tests already use — the radio, the switch,
**View Logs** with the tray off as readily as with it on, the greyed position with
its note, the Mac arm's missing radio, and no bridge at all. The settings page's
route test walks the word list rather than a written-out list, so the new word is
a case there without the test being edited — which is the whole reason that list
exists.

## Acceptance criteria

- [ ] The card opens the pane at the path the word list gives it, and the existing
      route test covers the new word with no edit of its own.
- [ ] The viewer's suite drives the radio, the switch and **View Logs** through a
      stub bridge, and asserts the greyed tray position with its note and a Mac's
      missing radio.
- [ ] **View Logs** opens the log file with the tray off as readily as with it on.
- [ ] Without the bridge the settings page draws exactly what it drew before, in a
      browser and on a phone both.
