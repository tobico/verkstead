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
electron-builder** into the same three artifacts, unsigned as before (on
Windows for good, *Amended: the Windows build stays unsigned*, below): a
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

Amended: **the Mac app is signed with a Developer ID and notarized**, by
[ADR-0027](0027-cluster-updates.md), *The release*: Squirrel.Mac refuses an
ad-hoc signed update, so a Release's dmg is now signed, notarized and stapled,
under the hardened runtime, with a `zip` of the same app beside it. A pack
given no certificate still signs ad-hoc, as every pack did before. The
AppImage and the Windows installer stay unsigned.

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

## Amended: the Windows build stays unsigned

Decided in the grilling of 2026-10-24 (Sets 33, 34 and 35).

**Packaging kept the three artifacts unsigned as before, and ADR-0012 had
called signing a cost decision to revisit; it was revisited, and the msi stays
unsigned.** The maintainer is an individual in Australia, part of no
organisation, and will not form a company to take a warning off Windows. As of
October 2026 that rules out the one route Microsoft itself offers, and every
other route was weighed and declined for a reason of its own, set out below.
What follows is the rule and the reason for each rather than its price list:
the prices are rough and the country lists move, and it is the eligibility
rules and the fit with `release.yml` that will still be worth reading next
year.

**Signing would not have taken SmartScreen away.** An EV certificate no longer
skips it — Microsoft ended that bypass in 2024, and its own documents say
Artifact Signing builds reputation over time as well — so every signed route
still starts at *Windows protected your PC*, and the window fades only as
downloads accrue to the publisher that signed them. What the money buys first is
the blue window naming a publisher instead of *Unknown publisher*. So the
README's steps, **More info** then **Run anyway**, are the same steps a
newly signed build would have needed for its first months.

**It reopens on any of three things**: a company in Australia, which can
validate for Azure Artifact Signing as an organisation; Microsoft opening
individual validation to Australia; or Verkstead becoming widely used enough
to show it on SignPath Foundation's application. Nothing else found changes
the answer.

**And if it reopens, it lands like this.** Sign the bare
`verkstead-windows-x64.exe` in its own matrix leg, so the sidecar packed into
the msi is the same signed bytes the release ships beside it — this ADR makes
the sidecar the released artifact. Let electron-builder sign the launcher and
the msi in the `desktop-windows` leg, which it does unasked once a signing
configuration is given: `signApp` signs the launcher, and the msi target calls
`signIf` on the artifact. Set `forceCodeSigning`, so a leg that cannot sign
fails the release rather than shipping unsigned again; and assert that
`Get-AuthenticodeSignature` reads Valid on the msi, the launcher and the
installed CLI. That is a pointer, not a design.

Per-user install is unaffected, and the reason the comments give for it still
holds: an installer asking for administrator would be an unsigned program
asking for the machine. The Mac build is unchanged as well; notarization was
not taken up.

### Considered Options

- **Azure Artifact Signing** (Trusted Signing until Microsoft renamed it in
  2026): about US$10 a month, what Microsoft recommends for an app outside the
  Store, and driven natively by the electron-builder `desktop/` pins through
  `win.azureSignOptions`. Rejected: Microsoft validates individuals only in
  the United States and Canada, and an organisation — which can validate from
  Australia — is a company the maintainer will not form.
- **SignPath Foundation**, which signs open-source projects free under its own
  certificate through its GitHub Action, the publisher a downloader sees being
  *SignPath Foundation*. Verkstead meets the published terms — an OSI licence,
  a public repository, free downloads, active maintenance, a release already
  made, no malware — but the application form also requires links or
  information showing that the project is widely used or trusted. Rejected:
  Verkstead has none to give yet.
- **SSL.com's Individual Validated certificate with eSigner**, sold worldwide,
  whose command-line tool and GitHub Action sign headless from a username, a
  password and a TOTP secret, so it fits a hosted runner. Rejected on cost: a
  few hundred dollars a year, certificate and cloud key together.
- **Certum's Open Source certificate on SimplySign**, about €50 a year for an
  individual, the publisher shown as *Open Source Developer* and a name. Its
  cloud key is a virtual smart card presented through a Windows desktop client
  with a phone OTP, and nothing found says it runs unattended, so it would mean
  signing by hand or a self-hosted runner — which `release.yml` is built not to
  need. Rejected on fit as well as cost.
- **A certificate on a USB token.** The CA/Browser Forum has required the key
  in a hardware module since 2023, and a token cannot be plugged into GitHub's
  runners. Rejected.

## Amended: a download offers to install itself

Decided in the grilling of 2026-10-29 (Sets 1515 to 1517).

**A Mac app run from the dmg or from Downloads, and an AppImage run from
wherever it was saved, offer to install themselves.** Packaging above left each
as the thing a human downloads: the README told a Mac to drag the bundle into
Applications and a Linux desktop to make the file executable and run it, and
nothing in the app knew where it was running from. What that left was this: a
Mac app run from the image, or from a Downloads folder it was quarantined into,
is run by Gatekeeper from a read-only randomised path, where the Update
[ADR-0027](0027-cluster-updates.md) gave it cannot land; and an AppImage in
`~/Downloads` is one whose Launch on startup entry and self-update both point at
a file in Downloads, with no entry in the applications menu. Windows is
untouched, by decision: its download is the installer (ADR-0027), and the Brief's
naming of it was a slip the human corrected.

**Where it fires.** A packed app running from outside an Applications folder —
Electron's own `isInApplicationsFolder` on a Mac, `$APPIMAGE` not under
`~/Applications` on Linux, which is where AppImageLauncher and Gear Lever put
theirs too, so a human who already integrated it is left alone. One rule on both
platforms; Downloads and the dmg alone, by path, was the Brief's wording and was
not taken, the detection being more fragile and a human who keeps AppImages in
`~/bin` seeing the offer once and ticking the box. Never on a checkout run, and
never on a hidden start: a login's start has nobody at a window that never came
up, and the offer waits for the next shown launch. In the lifecycle it comes
after the single-instance lock and before the port probe and the sidecar, so
nothing has been started when the app hands over to the copy — and the relaunch
needs no wait of its own, Electron's relaunch and its Mac move both waiting for
this process to exit before the new one starts.

**The dialog** is the platform's own message box with its own checkbox, as the
two dialogs the app already has are native: *Install Verkstead?*, a sentence
saying where it runs from and what installing does, **Install** and **Skip**,
**Don't ask again**. Skip asks again at the next launch; the box writes a fourth
key into `desktop.json`, read one key at a time like the three before it. A card
drawn by the page was rejected: it would need a window up and a bridge channel
for a question the app can ask before either exists.

**Install on a Mac** is Electron's `moveToApplicationsFolder` into
`/Applications`: it moves, trashes the copy that was there, asks for the
administrator's password where the folder wants one, and relaunches on its own.
`~/Applications` by a copy of Verkstead's own was rejected: `/Applications` is
where the README has pointed and where the release leg installs the bundle to
prove it. The dmg is left mounted. A password dialog cancelled is a move that
could not be made, and gets the failure dialog below rather than a silent skip:
the human pressed Install, and an app that said nothing about not having done
it would be the one hiding something.

**Install on Linux** moves `$APPIMAGE` to `~/Applications/Verkstead-x86_64.AppImage`
— the release's own name, whatever a browser renamed the download to — keeps it
executable, writes `~/.local/share/applications/net.tobico.Verkstead.desktop`
naming the copy, quoted the way the autostart entry quotes its path, copies the
icon set out of the mounted image into `~/.local/share/icons/hicolor`, and
relaunches from the copy. The icons because an entry whose `Icon` names nothing
draws the generic launcher icon, and the only icons are inside an image that is
gone once the app is not running from it; one png beside the file was rejected
for the set. Move rather than copy, on both platforms: a download left behind is
one that shows the offer again if it is ever run. The Launch on startup entry
needs nothing done to it — every launch rewrites it with the running path while
it is there, as the **Startup Registration** entry says.

**A Verkstead already installed there** has its version read first:
`CFBundleShortVersionString` out of the bundle's `Info.plist`, and
`X-AppImage-Version` out of the desktop entry electron-builder writes into the
image's root, which the AppImage runtime extracts without starting anything, so
a release from before this one answers too — a `--version` flag of the app's own
was rejected for that reason. Newer than the one running: *Open the installed
Verkstead?*, with **Open installed** relaunching it and ending this one, and
**Keep this one** carrying on. Older or the same: replaced, a pre-release
counting as older than its release. An installed copy that is *running* never
reaches this — the single-instance lock has handed the launch over before any
dialog could be drawn.

**Failure** — no space, a folder that cannot be made — is an error dialog saying
why, and the app carries on from where it is; quitting was rejected.

**And the Desktop settings page is the way back.** A row drawn only while the
app runs from outside Applications, saying where it runs from, with an
**Install** button: the dialog's Install over one more bridge channel, raising
the same dialog with one more sentence when sessions are running on this device,
naming them as stopped and resumed, the way the Update card names them. Without
the row, Don't ask again would leave hand-editing `desktop.json` as the one way
to install later, which is not a control. Refusing while a session runs, and
installing from the page with no dialog, were both offered and not taken.

**And `--install` is the dialog's Install with nobody asked**, an argument of
the app's own beside `--hidden`: run from outside Applications it installs and
relaunches as the press would, and run from inside it does nothing but say so.
It exists because a native dialog cannot be pressed on a runner: the release
legs install with it the way a downloader does — the Linux leg from a Downloads
of its own into a home of its own, asserting the file, the entry, the icons and
the relaunch; the Mac leg out of the mounted image in place of the `ditto` into
`/Applications` it did by hand, asserting that the relaunched app runs from
`/Applications` and is not translocated. Seeding `desktop.json` with the box
ticked before a shown launch, and proving the move by hand alone, was the
alternative and was not taken.

### Considered Options

- **Windows too.** Rejected: the exe is the installer.
- **Only the dmg and Downloads, by path.** Rejected for one rule on both
  platforms, the box being the way out for a human who keeps AppImages
  elsewhere.
- **`~/Applications` on a Mac, by a copy of Verkstead's own.** Rejected for
  Electron's call into `/Applications`.
- **Copy, and leave the download.** Rejected: one Verkstead afterwards.
- **Replace an installed copy whatever its version.** Refined by the human: a
  newer one is offered to open instead.
- **A card drawn by the page.** Rejected for the native dialog.
- **A file of its own for Don't ask again.** Rejected for a fourth key.
- **Nothing beyond the dialog.** Rejected for the row on the Desktop page.
- **Quit when the move fails.** Rejected for carrying on.
- **A cancelled password dialog as a silent skip.** Rejected for the failure
  dialog.
- **No `--install`, the legs seeding the box ticked.** Rejected: the move
  would be proven by hand alone.
