# 03. Tray, close policy and the Desktop page

## Goal

The app puts an icon in the tray with **Open**, **View Logs** and **Quit**,
and a new **Desktop** section at the top of the settings — drawn only inside
the app — holds **When the window is closed** (keep running in the tray, the
default; ask before quitting; quit), **Show tray icon** and **Launch on
Startup**. Closing the window does what the radio says; turning the tray off
greys the tray position and the choice falls to Quit; the tray's Quit never
asks. Launch on Startup writes the XDG autostart entry on Linux and a login
start comes up hidden while the tray is shown. Proven on COSMIC and against a
panel that arrives after the app.

## Decisions in force

- **One radio, two checkboxes** ([ADR-0020], Set 846 Q7): the Brief's two
  mutually exclusive checkboxes are one control. The warning is a native
  dialog saying running sessions will be stopped, with Quit and Cancel.
- **Tray off means the choice falls to Quit** (Q7a), the tray position greyed
  with a note saying why. Not "ask first": a hidden app with no icon is a
  Verkstead nobody can reach.
- **The warning is the close button's alone** (Q7b): the tray's Quit and
  Cmd+Q quit at once.
- **The tray menu is Open, View Logs, Quit; a left click opens the window**
  (Q10). Launch on Startup leaves the menu for the page, as the Brief says.
- **Desktop settings are the app's** (Set 847 Q12): a JSON file under
  Electron's user data, reached over a preload bridge exposing the platform,
  the settings and the startup registration. The page is drawn only where the
  bridge exists, so a phone never sees it; nothing new is on the wire or in
  `config.yaml`. The per-device push switch is the pattern.
- **Launch on Startup stays "the registration is the state"** (Q12a, Set 846
  Q9a): read from the platform through the bridge, written to it, never copied
  into the JSON. An XDG autostart file on Linux; the login-item API arms for
  a Mac and Windows are written here as arms and proven in stages 06 and 07.
- **A login start is hidden while the tray is shown, and shown otherwise**
  (Q9). Not always shown, and not a Launch on Startup that needs the tray.
- **The Linux tray is Electron's own, accepted as a risk** (Set 847 Q13):
  this stage proves it on COSMIC and against a late panel, and Show tray off
  is the way out. Chromium speaks StatusNotifierItem itself and watches for
  the panel's name, which is what makes the late-panel case plausible.
- **The page is the settings page's own kind of section**: a card in the
  middle pane above the git card, a word added to the route list so the route
  and its test arrive with it, and the details pane's switch grows an arm.
- **Proof** (Set 848 Q17 and Q17a): the close policy, the settings file and
  the autostart arm under vitest; the page under the viewer's suite with a
  stub bridge beside the stubbed fetch.

## Proposed tasks (provisional)

1. **The bridge and the settings file** — preload exposes `platform`, get and
   set over the JSON, and the startup registration; main enacts them.
   Accepts: a set survives a restart; a page with no bridge sees nothing.
2. **The tray** — icon from the packaging artwork, the three items, click
   opens. Accepts: a hidden window returns on Open and on click; View Logs
   opens the file or says there is none; Quit quits without asking.
3. **The close policy** — the three positions enacted on the window's close,
   the fall-through when the tray is off, the native warning. Accepts: each
   position does what it says; tray off with "keep running" chosen quits.
4. **The Desktop page** — the card at the top of the settings, the route
   word, the details pane with the radio and two checkboxes, greyed states
   with their notes, drawn only with the bridge. Accepts: the route test
   covers it; the viewer suite drives it through a stub bridge; without the
   bridge the settings page is as it was.
5. **Launch on Startup on Linux, and the hidden login start** — the XDG entry
   written with the app's own path and a hidden flag, rewritten while present
   at every launch as the tray app did, read for the checkbox. Accepts: the
   entry appears and disappears with the box; a start with the flag shows no
   window while the tray is on and shows one when it is off.
6. **COSMIC and the late panel** — run on a COSMIC session and with the panel
   started after the app. Accepts: every item activates; the icon appears
   when the panel does. What fails here is written into the brief for 05.

## Re-verify at start

- Stage 02 landed: the window, the sidecar and the log file exist.
- The settings page still composes its cards by hand in `SettingsPage.tsx`
  and its routes from the `WORDS` list in `openings.ts`.
- What the tray app's XDG arm writes today — the `Hidden` and
  `X-GNOME-Autostart-enabled` readings, the AppImage `$APPIMAGE` case — so the
  Electron arm reads the same entries a human may have edited.
- Whether Electron's tray API on the pinned major still creates the icon
  before a StatusNotifierWatcher exists, or throws.

[ADR-0020]: ../../adr/0020-electron-desktop.md
