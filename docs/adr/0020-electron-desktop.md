# An Electron desktop app over a sidecar server

Supersedes [ADR-0012](0012-desktop-tray-binary.md): the desktop app is a
window rather than a tray over a browser, and it is not written in Rust.

ADR-0012 put Verkstead on the desktop as lifecycle alone — a tray icon over
an in-process server, with the browser the human already had as the whole of
the interface — and rejected Electron and Tauri by name for the weight they
add to a UI the browser renders from localhost. What that bought was a few
megabytes over the CLI and a pure-Rust toolchain. What it cost turned out to
be the desktop itself: the workbench lived in a browser tab beside everything
else, the tray was four items that a tab cannot be, and every platform's
toolkit had to be spoken to by hand — GTK, AppKit and Win32 for two dialogs
and a menu, a StatusNotifierItem implementation of Verkstead's own after the
library one drew a menu on COSMIC that activated nothing. So the desktop app
becomes **an Electron application: a window of its own over the workbench,
the server run beside it as a sidecar, and the platform's tray, dialogs and
startup registration reached through one toolkit rather than three.**

Decided in the grilling of 2026-09-25. The roadmap that builds it is
`docs/roadmaps/electron-desktop/`.

## The sidecar is the CLI

**The app carries the headless `verkstead` binary and starts it as `verkstead
serve --desktop`.** Nothing of the desktop is in Rust any more: `crates/desktop`
goes, and with it the CLI's default-on `desktop` cargo feature and verb, the
Windows shim, the macOS launcher script and the AppImage's `AppRun`. The
binary is one slim artifact again on every platform, which is what
[ADR-0004](0004-single-binary-distribution.md) wanted before ADR-0012's
amendment had to bend it — and the headless CLI ships exactly as it does
today, as the separate package it always was.

The invariant that amendment guarded holds without effort. The Sandbox binds
the running server's own image first on a session's PATH so the two halves of
an ask cannot skew, and a sidecar that *is* the CLI is that image: every
session started under the app asks with the very binary that serves it. What
the app bundles is the CLI artifact the release already built — the static
musl binary on Linux, the two Mac builds joined with `lipo`, the Windows exe —
downloaded from the same run rather than compiled a second time. On Linux that
removes the glibc floor the AppImage carried: nothing in the sidecar links a
toolkit now, so the only floor is Electron's own.

**The flag is what the server knows about who started it**, and it changes
two things. The startup line names the address alone, because the log under
the app is a file on somebody's desk that a menu item opens
([ADR-0015](0015-open-boundary-and-workbench-key.md), as amended). And a
refused `tailscale serve` raises the platform's own password dialog rather
than handing back a `sudo` line: the three arms of that asking — `pkexec`,
`osascript`, PowerShell's `Start-Process -Verb RunAs` — were never anything
but a spawned command, so they move into the server crate as they are and the
flag is what turns them on. Nothing about the flag reaches the wire: the
viewer learns it is inside the app another way, below.

**Electron holds the key the way the tray app did.** The Workbench Key is
made by the server at first start and kept in the Data Directory; the app
waits for the server's health, reads the file, and opens its window on the
login link, so the window lands logged in. A 401 on the window's own frame is
the key having been reset from the phone, and the app reads the file again
and reloads. The address stays `127.0.0.1:8422` and a foreign listener there
is still a dialog and an exit, never a server fronted or a port picked; what
Electron adds is a single-instance lock, so a second launch of the *app* is
the window brought forward rather than that dialog. The server ending is the
app quitting. And the log file is the app's to write: the sidecar logs to
stdout as `verkstead serve` always has, and the app puts those lines beside
its own in `verkstead.log` under the Log Directory, with the byte-order mark
and the roll at a few megabytes the Rust app gave it.

## The window

**One window, decorated by the page.** It loads the workbench off the
loopback with nothing about the viewer changed to draw inside it, and it has
no title bar: the platform's own controls stand where one would have been —
the traffic lights inset to the left of the Wordmark on a Mac, the controls
overlay at the top-right corner on Windows and Linux, coloured and sized from
the page and recoloured when the scheme flips — and the pane heads are the
drag region, their buttons excepted. The frame is what keeps the outermost
heads clear of the controls, reading the overlay's rectangle, which is also
what a narrow window's single pane gets. A page with no pane head — the
onboarding wizard, the no-such-page, and the moment before the verdict about
this machine lands — draws a bare drag bar of the same height, inside the app
only. The menu bar is hidden on Windows and Linux with its shortcuts kept; a
Mac's is filled out with Verkstead's own menus (*Amended: the Mac menu bar*,
below). Links off the workbench
open in the system browser, and the window never navigates away from
localhost. Its size and position are remembered between runs.

**Closing the window is a choice, and the choice is one control.** The Brief
named close-to-tray and warn-before-closing as two checkboxes that exclude
each other, which is a radio: **When the window is closed** — keep running in
the tray, ask before quitting, or quit — with keep running the default. Beside
it stand **Show tray icon**, on by default, and **Launch on Startup**. Turning
the tray off greys the tray position and the choice falls to Quit: a hidden
app with no icon is a Verkstead nobody can reach. The warning is the close
button's alone — the tray's Quit and Cmd+Q quit at once, because somebody who
chose Quit has already said what they meant. The tray menu is Open, View Logs
and Quit, and a left click on the icon opens the window.

**View Logs is the page's as well as the tray's.** A switch somebody can turn
off cannot be the only way to a log file, and the desktop most likely to have
it turned off is the one the tray misbehaved on — which is exactly the machine
whose log is worth reading. So the Desktop page carries a **View Logs** button
beside the switches, on every platform, opening the file the tray item opens;
the tray keeps its item for as long as there is a tray.

**A Mac is the platform's own.** The app is a regular Dock app there, the
menu-bar-only policy the tray app asked for gone: closing the window leaves
the app in the Dock, and a Dock click brings the window back. So the close
radio has nothing to choose on a Mac and is not drawn, and neither is the
warning; the Desktop page there holds the menu bar icon — on by default, one
rule everywhere — View Logs beside it, and Launch on Startup.

**A login start comes up hidden while the tray is shown**, the way `--no-open`
kept a login from being handed a browser window, and shown when it is not.
The registration is still the state rather than a copy of it: Electron's
login-item API on a Mac and Windows — the app is always a bundle now, which is
what the tray app's hand-written plist was working around — and an XDG
autostart file on Linux, where Electron has no such API.

**And the tray app's own registration is taken over rather than orphaned.**
Linux needs nothing for this, the entry being the same file under the same
name; the other two change mechanism, and a registration the new API cannot
see is one that goes on firing at a path this roadmap deletes while the box
above it reads off. So the first launch after an upgrade reads what the tray
app wrote — the launch agent at
`~/Library/LaunchAgents/net.tobico.Verkstead.plist`, the Run value named
`net.tobico.Verkstead` — carries whether it was on into the new registration,
and deletes it. Once, at launch, and never as a control: it is this app's own
registration under an older name rather than a second setting.

## The Desktop page

**Desktop settings are the app's, not the server's.** They are kept in a
JSON file of Electron's own under its user data, and reached from the page
over a preload bridge that exposes the platform, the settings, and the
startup registration. The Desktop page stands at the top of the settings and
is drawn only where the bridge exists — so a phone on the tailnet never sees
it, and nothing new is on the wire or in `config.yaml`. That is the split the
per-device push switch already made: a fact about the machine in front of the
human, never sent to the server.

## The tray on Linux, as a risk

Electron's tray speaks StatusNotifierItem through Chromium's own D-Bus code
rather than through the library the Rust app moved off, and Chromium watches
for the panel's name, so a panel that arrives after the app at login should
still get the icon. None of that is verifiable until it is built. The
decision is to accept it: the stage that builds the tray proves it on COSMIC
and against a late panel, and **Show tray icon** off is the way out on a
desktop that misbehaves.

## Packaging

**A top-level `desktop/` pnpm project beside `web/`, built by
electron-builder** into the same three artifacts, unsigned as before: a
universal dmg, an x86_64 AppImage, and an msi through electron-builder's WiX
target with a fragment of our own putting the CLI on the user's PATH, which
today's msi does and the adoption docs promise. What goes on it is the CLI's
own directory inside the install rather than the install root: the root holds
a launcher electron-builder names for the product, Windows resolves `PATH`
without regard to case, and a `verkstead` that started the window instead of
printing the guide would be the promise kept backwards. Each release leg
launches the packed app and asserts what it asserted of the tray app — it
serves, the binary inside answers `ask`, and its log says the window and the
tray came up. The main process's pure parts are under vitest, and so is the
page with a stubbed bridge. nixpkgs' Electron joins the dev shell for running
the app locally; the flake stays daemon-only.

Amended: **the `PATH` entry is the msi target's `msiProjectCreated` hook
rather than a fragment of our own.** There is nowhere for a second WiX source
to go — the target reads its template out of `app-builder-lib`, hands candle
one `project.wxs` and light one `project.wixobj`, and takes no override for
either. What it does take is that key, naming a JS file it runs after the
project is written and before candle compiles it, so the entry and the install
directory's name beside it are patches of the generated project instead —
`desktop/scripts/msi.mjs`. String surgery on a generated file, so each patch
has to match exactly once or the pack stops rather than writing an installer
with no `PATH` entry in it. What goes on the `PATH` and why is unchanged; the
mechanism this paragraph named for it was one electron-builder has not got.

## Considered Options

- **Keeping `verkstead desktop` beside Electron**, or shrinking the crate to
  what Electron cannot do. Rejected: two desktop products, and nothing left
  that Electron cannot do.
- **Plain `verkstead serve` with Electron filtering the key out of the log**
  and the grant left as a `sudo` line. Rejected for a regex over a log line
  standing between a secret and a file on a desk.
- **The sidecar printing the login link on a machine-readable line**, or
  Electron generating the key and handing it in. Rejected: the file is the
  documented thing, and the server already makes it.
- **A free port when 8422 is taken.** Rejected as ADR-0012 rejected it: the
  CLI's default and `tailscale serve` both name the port.
- **Custom window controls drawn by the page.** Rejected: native controls
  keep Windows' snap layouts and a Mac's traffic lights as the platform draws
  them.
- **Desktop settings in `config.yaml` behind a server flag.** Rejected: a
  setting the server never enacts, put on the wire so a phone could flip it.
- **Three checkboxes as the Brief worded them.** Rejected for the radio: two
  boxes that exclude each other are one control.
- **An NSIS installer** in place of the msi. Rejected by the human: the msi
  stays.
- **Electron Forge**, or the project as a package inside `web/`. Rejected for
  electron-builder's three targets in one configuration, and for a project
  that consumes a built binary rather than sharing a toolchain.

## Amended: notifications are the app's own

Decided in the grilling of 2026-10-12 (Sets 1018 and 1019).

**Web Push cannot reach the window.** The switch on the Settings head offered
it inside the app because every check passes there — the loopback is a secure
context, the service worker registers, `PushManager` exists — and then the
subscribe is refused inside Chromium with *push service error*, the refusal a
de-Googled browser gives, because Electron ships Chromium without Google's
push client. No key, no server change and no retry gets past it: there is no
push service for a loopback to talk to.

So **the sidecar tells the app directly, and the app shows the platform's
own notification.** The one function in the server that every push already
passes through — a Set arriving, a stop, a session gone idle, an account out
of window or signed out, a pull request, a roadmap moving on, Done, a device
asking to join, and a member's relayed news — announces each notice on a local
stream as well, `GET /api/ui/push/notices`, under the prefix never served over
the cluster link, carrying exactly what a phone gets: the sentence, the path a
tap opens, the repo. The main process holds that stream for the app's whole
run, logged in with the Workbench Key the way the window is, and shows each
notice as a native notification: the sentence as the title, the repo as the
body, the platform's own sound. A notice while the stream is down is lost, as
a push is when its service is: nothing is queued and nothing is replayed, and
the Timeline says it in full either way.

**Shown only while the window is not in front** — hidden to the tray,
minimised, behind another app, or never shown on a login start. A focused
window already draws the news, which is the one thing a desktop can tell that
a phone cannot. **A click brings the window forward** the way the tray's Open
does and opens the notice's path in the page already loaded, over a channel
from the app to the page — the first on the bridge in that direction with an
answer to act on — so nothing reloads; a window whose page is not there is
loaded on the path instead.

**The switch is the app's setting inside the app, and on by default.** The
same switch on the Settings head, which outside the app is the Web Push switch
it always was, reads and writes `notifications` in `desktop.json` over the
bridge — beside the close policy, and for the same reason: whether this desk
lights up is a fact about the desk, never sent to the server. No browser
permission stands behind it. Where the platform cannot show one, the line
under the switch says so; a refusal in the OS's own settings is the OS's to
lift. **Everything the phones get**, a member's news and a join included: one
function sends it all, and a desktop that was told less than a phone would be
one the human learns not to trust. **And a Mac asks its permission at the
first notification** (Set 1021 Q9): Electron can raise that prompt only by
showing one, a launch asks nothing of the human, and a welcome shown at the
first Conversation started was rejected as a notification about nothing.

### Considered Options

- **The page showing them, off its own Nudge stream.** Rejected: the page
  drops its stream when it is not being looked at, which is exactly when a
  notification is worth showing, and a Nudge carries no sentence by design
  (ADR-0009).
- **Making Web Push work in Electron with a third-party FCM receiver.**
  Rejected: Firebase credentials, Google in the loop, and a round trip to the
  internet for a message between two processes on one machine.
- **A separate checkbox in the Desktop section.** Rejected: the head's switch
  already means *tell this device*, and two controls for one setting are one
  too many.
- **Showing every notification, as the phone does.** Rejected for the focus
  rule: a toast over the very page that just drew the news is noise.

## Amended: the Mac menu bar

Decided in the grilling of 2026-10-20 (Sets 1281, 1283 and 1284).

**The bar above kept only what a Mac cannot do without**: the application menu
as Electron fills it in, Edit, View and Window, every item a role. So there was
no Settings… on ⌘, — the one key every Mac app answers — no File menu, so ⌘W
closed nothing, and nothing in the bar went anywhere in the workbench. **On a
Mac it is now filled out**, and Windows and Linux keep their hidden bars exactly
as they were, item for item:

- **Verkstead** — About · **Settings…** `⌘,` · Services · Hide · Hide Others ·
  Show All · Quit. Written out rather than the `appMenu` role, which nothing
  can be added to, so that Settings… stands where a Mac puts it; every other
  item is still the platform's role. It opens the settings page itself, and it
  is *Settings…* because that is what macOS 13 renamed it to and what the
  workbench calls the page.
- **File** — **New Conversation** `⌘N`, the compose page, and **Close Window**
  `⌘W`, the platform's `close`, which the close policy already makes a hide to
  the Dock, the way the red light is.
- **Edit** and **View** — as they were.
- **Go** — **Back** `⌘[` · **Forward** `⌘]` · **Conversations** ·
  **Timeline** `⌘1` · **Code** `⌘2` · **Backlog** `⌘3` · **Roadmap** `⌘4` ·
  **Share** `⌘5`. Conversations is the workbench with nothing open and carries
  no key, ⌘0 being the zoom's reset; the five panes are the workbench's own
  order on the keys a Mac gives an app's main views.
- **Window** — the platform's own `windowMenu`.
- **Help** — the `help` role, which is what gives it a Mac's search field:
  **Verkstead Help**, the docs site in the browser; **View Logs**, the tray's
  act reached a third way; and **Report an Issue**, the issue tracker in the
  browser.

**In Title Case**, the platform's own casing rather than the viewer's sentence
case: the bar is a Mac's furniture, and an item that read differently from
Hide Others beside it would be the one that looked wrong.

**An item that goes somewhere sends a path to the page** over the channel a
clicked notification already crosses on, so the page routes there in place:
nothing reloads, and a brief half typed into the composer survives ⌘, and is
there again on Back. A window whose page is not listening is loaded on the
path instead. The menu is set before the window exists, so each act reads the
window at the press rather than holding one from the build.

**The Go menu is enabled by what the page says, the way the head's colours
are pushed.** Which Conversation is open, which device it is on and what its
record holds are the page's to know, so the page builds the list — each pane
the open Conversation has, with the path it routes to for it, a member's under
its device — and pushes it over the bridge whenever it changes. A pane the
page names nothing for is greyed: all five on the settings and compose pages,
Backlog where the record has no task list, Roadmap where no roadmap has
landed. The app checks what arrives before holding it, as it checks a set
from the page, and a press reads the latest push.

**Back and Forward are the window's own history**, which on a page that
routes in place is every pane and page walked through, greyed where there is
nothing behind or ahead. ⌘[ and ⌘] are also the Code pane's outdent and
indent, chosen knowing it: Electron gives the page the first refusal of a
keystroke, so a writable editor with the focus keeps them, and Back and
Forward fire everywhere else, a read-only editor included — the way Xcode
does it. Proven on a `macos-15` runner rather than taken on trust.

### Considered Options

- **The `appMenu` role with Settings… somewhere else.** Rejected: ⌘, in the
  application menu is where every Mac app keeps it.
- **The app asking the page where it is**, or working it out from the URL.
  Rejected: the page already knows, and a second copy of the routes in the
  main process is one that drifts.
- **⌘0 for Conversations.** Rejected: it is the zoom's reset, a role the View
  menu already carries.
- **Sentence case, as the viewer writes.** Rejected for the platform's casing
  in the platform's bar.
