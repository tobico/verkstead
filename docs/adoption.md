# Adoption

The switch-over: what Verkstead replaces, and how a day's work runs through it.
Working *on* Verkstead is [development.md](development.md); this is working
*with* it.

Until this point the loop was three tools kept in step by hand — askance for
the questions, `tobico-skills/roadrunner` for the driving, and the
`tobico-scripts` wrappers for the sandbox. They are what built Verkstead, stage
by stage, and they are what it replaces. Starting a piece of work stops meaning
*which of the three does this part* and starts meaning *open the workbench*.

**Where this stands.** The switch-over has been made for this repository:
Verkstead drives its own work, and it is what executes this roadmap — a Stage
of it is a Conversation, planned and built by sessions Verkstead starts. What
has *not* happened yet is a Conversation reaching **Done**. Every run so far
has been driven by hand past the finish: the step that pushes and opens the
draft pull request has never carried a Conversation into Wrapping, so the
wrap-up loop and the settling below it are still proved by the test suite
alone. Closing that is [stage 05](roadmaps/mvp/05-refinement.md), which is
being built the way everything else here is — by Verkstead, through this page.

The vocabulary in bold is the project's, defined once in
[CONTEXT.md](../CONTEXT.md).

## What it replaces

| Before | Now |
| --- | --- |
| `sandbox` / `work-sandbox` — bwrap around the whole of `~/src` | A **Sandbox** per **Conversation**: its **Worktree**, its Repo's git directory, its handoff directory, a **Built Root** made out of the **Agent Profile**'s account — its login, its memory and transcripts where the Profile shares them, and a configuration file of Verkstead's own — and nothing else of the machine |
| `agent`, `grilling`, `next-stage`, `next-tasks` — one wrapper per thing you might start | One **Conversation**, which runs through Draft → Grilling → Direction → Implementing → Wrapping → Done |
| `roadrunner` — a terminal per run, driving `.tasks/` and `docs/roadmaps/` | The orchestrator, driving the same two files off the Repo, with the run visible on a **Timeline** instead of scrolling past |
| roadrunner's interruptions | A **Halt** and its stop **Notice** — pushed to your phone, read where the work is, and answered by one **Resume** |
| askance — one queue of Question Sets for the machine | **Question Sets** on the Timeline of the Conversation they were asked from |
| The skills installed under `~/.claude/skills` | **Skills** shipped inside the binary and read-only inside at a path no backend owns, with the account's own not in a session's `~/.claude` at all, so a session's behaviour is the product's |
| A gate at every commit | No commit gates. Review consolidates in the wrap-up, per pull request |

What stays: **askance is a separate, maintained product**, and the
`tobico-skills` skills stay installed for ordinary terminal work in
repositories Verkstead is not driving. What retires is roadrunner and the
wrappers that launched it — see [The old tools](#the-old-tools).

## Getting it running

Nothing has been released under this name yet, so what follows is what a
release produces rather than something to fetch today —
[releasing.md](releasing.md) says what a release builds and where it puts it.

There are four ways in, and they are four different things rather than four
spellings of one. **The flake and the NixOS module run the headless daemon**, on
a machine that is always on and answering from wherever you are. **The AppImage
is the same server started from an icon**, on the Linux desktop in front of you,
with the workbench in the app's own window and a tray icon beside it. **The dmg
is that same server for a Mac**, the workbench in a window there too, with a
tile in the Dock and an icon in the menu bar beside it. **The msi installs that
same app on Windows**, into your own profile and without asking for
administrator. Which one you want is which of those machines you were
describing; two at once is two Verksteads, and the second to reach port 8422
says so in a dialog and exits.

### The daemon, on NixOS

On a NixOS host, import the flake and enable the service:

```nix
services.verkstead = {
  enable = true;
  paths = [ "/home/you/src" "/home/you/.claude" ];
  home = "/home/you";                 # optional; the service's own by default
  sandboxBinds = [ "/var/cache/verkstead-node" ];
};
```

Three of those are worth understanding before the first Conversation:

- **`paths`** is the list of directories bound read-write into the unit — the
  repositories it is to work in and the agent accounts it is to run under,
  alike. It says nothing to Verkstead: the server is never told the list
  exists, and inside the unit every path it is given is treated alike. What it
  is for is that `ProtectHome` and `ProtectSystem` hide everything the unit is
  not told to bind, so a directory not named here is not there as far as the
  service is concerned. **A repository or an account you did not name here is
  answered *missing*** — the same answer as a path that genuinely is not there,
  because inside the namespace it genuinely is not — and `paths` is where to
  add it. There is no default, no scan and no minimum: a build naming none of
  them is a legal build, and what it comes up as is a workbench with nowhere
  yet to point at.
- **`home`** is only what `HOME` means for the service; nothing is read out of
  it and nothing of it reaches a Sandbox. Credentials and identity are said
  instead: a token in `secrets.yaml` and a `git_author` in `config.yaml`, both
  in the data directory, reaching each session as `GH_TOKEN` and git's own
  `GIT_CONFIG_*`. It is bound in **read-only**, which is the whole of what
  naming one buys — so an **Agent Profile**'s account kept under it that has to
  be *written*, which is every account of every agent type, goes in `paths` as
  well. That is the one composition worth saying outright, and it is why the
  example above names `/home/you/.claude` beside the repositories; a Codex
  account would be `/home/you/.codex`, a Grok Build one `/home/you/.grok`, and
  an OpenCode one both `/home/you/.config/opencode` and
  `/home/you/.local/share/opencode`. A session is not given that directory
  whole: it gets a **Built Root** of Verkstead's own. Still, the account is
  written. A login or token refresh writes through to the login file. With the
  Profile's memory switch on, which is the default, memory and transcripts are
  written into the account's store — for Claude under two entries in
  `projects/`, which the server makes there first when they are missing. And
  for Claude, what a session changed in its copy of `.claude.json` is merged
  back into `/home/you/.claude.json` as it ends. That merge writes a new file
  beside the old one and renames it into place, so it needs the directory the
  file is in to be writable. In this example that directory is the read-only
  home, so the merge is logged and skipped; the session itself runs as normal.
  To have it carried back as well, keep the account somewhere of its own,
  named whole in `paths`, rather than in a home bound read-only.
- **`sandboxBinds`** is the **Sandbox Configuration** — every entry is a hole
  in the boundary, which is why one that is not there refuses startup rather
  than being skipped. Each is one absolute path, and every session gets every
  one of them.

The workbench says the binds as well, on the settings page's **Sandbox binds**
section. What a session gets is the union of the two. Those entries are saved into
`config.yaml` in the data directory, read afresh every time they are used, and
never fatal:
one the server cannot see is reported on the page rather than refused, and
simply covers nothing. **On this module, that report is the one to read** — the
unit's namespace holds what the options above name and nothing else, so a bind
typed into the settings page saves, says the server cannot see it, and does
nothing until it is added to `sandboxBinds` here too. A repository or an account
outside the namespace has no such report to read, only the *missing* it is
refused with, which is why `paths` above is the list to reach for when a
directory you can see from a shell is one the workbench says is not there.

A bare binary outside NixOS has no namespace like this and needs no options at
all: see [development.md](development.md#quickstart).

Three more are about the **Peer Listener**, which is the second listener and the
one other devices dial. **`peerListen`** is where it binds — `0.0.0.0:8423` by
default, every interface, because the device calling this one is on a LAN or a
tailnet and neither of those is the loopback — and **`openFirewall`** opens that
port on the host's firewall, on by default. That default is the one worth
knowing about: a NixOS host firewalls by default, and a peer listener nothing
can reach is a linking that cannot happen, with the device dialling in timing
out and nothing on either machine saying why. The workbench's own port stays
shut, because what reaches it from another device is `tailscale serve` on the
tailnet. A host that declares its open ports somewhere of its own sets
`openFirewall = false;` and opens what it needs there instead.

The third is how the two machines find each other in the first place.
**`advertising`** is whether this host says what it is on the LAN: an mDNS
advertisement of `_verkstead._tcp.local` carrying this device's id, the
machine's hostname, the word for its operating system and the port `peerListen`
named — which is what another Verkstead on the same LAN reads to find this one,
with no address for anybody to type. On by default, for `openFirewall`'s reason:
a discovery nothing can hear is a feature that silently does not work. Set
`advertising = false;` on a LAN that is not yours alone — a hostname, an
operating system and a device id is more than some networks are worth telling —
and link by a typed address instead.

**It turns off what other machines hear *of* this one, and not this one's
listening.** With it off, nothing of this host's hostname, OS or device id goes
out and no other Verkstead can find it — but its own **Discovered** list still
works, so opening the Remote access pane still puts a query for
`_verkstead._tcp.local` on the LAN from this machine's address. That is all the
LAN learns: that something here is looking for Verksteads, and not what it is —
no hostname, no OS and no device id. A host that is to say nothing at all on that
wire is one whose Remote access pane stays closed; `openFirewall = false;` shuts
the answers out rather than the question in.

`openFirewall` opens UDP 5353 for this beside the peer port, and opens it whether
or not the host advertises: the answers to its *own* browsing arrive there too, so
a host that only looks for others still has to be able to hear.

What device this install is reads in the **Devices** list at the foot of that
same **Remote access** section: its name with an icon for its OS, and the
addresses another device could dial it on. It holds this device alone — nothing
links anything yet — and the same two facts are on the startup line as `device=`
and `fingerprint=` for anybody reading a journal rather than a screen.

A build cache is not one of them, and there is nothing to configure for one.
The **Build Cache** is the server's own: the module makes
`/var/cache/verkstead`, puts `sccache` on the service's path, and every Sandbox
gets the directory writable with `CARGO_HOME` inside it and `sccache` as its
`RUSTC_WRAPPER` — so a crate is downloaded once and compiled once for the
machine rather than once per Conversation. The sccache server every Sandbox
compiles through is Verkstead's own, in a Sandbox of its own holding the
worktrees and the cache, and it comes and goes with the service. Which
languages a Sandbox gets at all, and how
large the compiled half may grow, are in the workbench settings; each is on with
nothing configured. `systemctl clean --what=cache verkstead` empties it, and
nothing but build output is in it. Those variables are not Rust's by name
anywhere in the server: Rust is a **descriptor** like any other, and writing
one for a language Verkstead has never heard of is [Languages](#languages)
below.

The **Data Directory** is not one of the three either, and not a choice on this
module: the unit keeps it in its own state directory, `/var/lib/verkstead`, and
passes it as `--data-dir`. Started without that flag — the same package run by
hand — Verkstead keeps it in the platform's own place instead:
`~/.local/share/verkstead` on Linux, `~/Library/Application Support/Verkstead`
on macOS, `%APPDATA%\Verkstead` on Windows. One directory either way, holding
the database, the Worktrees, the Skills, the handoff directories and both
settings files.

**The first visit is out of the journal.** Every page of the workbench answers
401 without the **Workbench Key** — the one secret Verkstead makes in its Data
Directory at the first start, which is what keeps a session out of the workbench
it is being watched through — and a host with no tray icon has one place to be
handed the link: the line the server logs as it comes up.

```console
$ journalctl -u verkstead | grep 'verkstead is listening'
  INFO verkstead_server: verkstead is listening listen=127.0.0.1:8422 workbench=http://127.0.0.1:8422/?key=… data_dir=/var/lib/verkstead …
```

`workbench=` is the address with the key on the end of it. Open it once and the
browser holds a cookie from then on; the same line is there after a restart, so
a device that forgot the cookie is let back in by reading it again.

The server binds loopback and speaks plain HTTP, and answering from a phone
needs HTTPS — which push notifications need to work at all. That is the
**Remote access** section of the workbench settings rather than anything to run
here: it reads what this machine's Tailscale is doing, a checkbox puts the
tailnet name in front of the port, and the login link is drawn there as a QR
code to point a phone's camera at. What this module does for it is the two
things a host has to do — `tailscale` goes on the unit's own `PATH`, and the
service user is made the tailnet's operator, so nobody is shown a `sudo` line
for a grant the build already made. Joining the tailnet stays the host's own
business: `services.tailscale.enable`, and a `tailscale up` in a terminal.

### The desktop app, on a Linux machine

`Verkstead-x86_64.AppImage` is one file holding the app, the server it starts
and the viewer the two of them draw between them, with Electron's own browser
runtime beside them — so a machine with none of that installed needs nothing
else to put the workbench on the screen. x86_64 only: an arm64 Linux desktop has
the bare CLI and `verkstead serve`. Downloaded, made executable — a Release
asset carries no mode — and run, it starts the server on `127.0.0.1:8422`, opens
the workbench in a window of its own, and puts an icon in the tray with the
three things the window cannot do for itself: **Open** brings the window back,
**View Logs** opens the file this run's logging goes to when there is no
terminal to print it in, and **Quit** stops the app and the server with it.

**There is nothing to learn about starting it.** The app takes one flag,
`--hidden`, and its own startup registration is what writes that rather than
anybody typing it; what it reads instead is the server's own environment, so
`VERKSTEAD_DATA_DIR` moves the Data Directory off `~/.local/share/verkstead`
exactly as it does for a `verkstead serve` run by hand. Everything else about
this machine is set rather than typed, on the **Desktop** section of the
settings page: **Launch on Startup**, **When the window is closed** — keep
running in the tray, ask first, or quit — and **Show tray icon**.

**The window it opens is logged in.** Every page of the workbench answers 401
without the **Workbench Key**, the secret Verkstead keeps in its Data Directory
where no session can reach it — and the app reads that file itself, before there
is a server to ask one of, so the window comes up on the address with the key on
the end of it. There is nothing to keep anywhere and nothing to type: **Open**
is that same window brought forward rather than a login handed over again. The
app's own startup line names the address and no key, because **View Logs** opens
a file on your desk and a workbench key written into it would be a login for
anybody reading over your shoulder.

**Answering from your phone is the workbench's own settings**, under **Remote
access**: it reads what this machine's Tailscale is doing, a checkbox puts the
tailnet name in front of the port — HTTPS, which push notifications need to work
at all — and the login link is drawn there as a QR code to point a camera at.
Nothing here installs Tailscale: the pane points at where to get one where the
machine has none, and joining a tailnet is a `tailscale up` in a terminal. What
the app adds over the daemon is the password dialog — Tailscale refuses to be
served by a process that is neither root nor the tailnet's operator, and an app
has somebody at the machine to ask, so that press goes through `pkexec` rather
than handing back a `sudo` line to type.

**What is inside is the app and the released `verkstead` beside it**, rather
than one binary whose entry point supplies a verb. The CLI sits in a directory
of its own under the app's resources, and it is the very build a Release
publishes — the statically linked binary the same run's CLI leg made, downloaded
into the package rather than compiled a second time
([ADR-0020](adr/0020-electron-desktop.md)). It is what a session started here is
handed to ask with, so the two halves of an ask are one build; and because that
binary is static, a session is handed the binary itself — no launcher in front
of it, and none of the bundle's own libraries anywhere on its way.

**The window has no title bar.** The workbench's own heads are the top of it,
and what stands where a title bar would have been is your platform's window
controls, inset at the top-right corner. Which of them are drawn is the
platform's answer rather than ours, and on a Wayland desktop Chromium draws the
close button alone — which is what a COSMIC session gets. So the gestures for
the rest are the app's own: a double-click on any pane head maximises the window
and a second one restores it, `Ctrl+M` puts it away, and closing it keeps
Verkstead running in the tray, which is what **When the window is closed** says
until you say otherwise. The menu bar is hidden and Alt is what brings it down,
with that same **Minimize** under **Window** where a keystroke was not what you
wanted.

**A desktop with no tray host loses the icon and nothing else.** Vanilla GNOME
is the case people meet — it draws no tray, and an AppIndicator extension is
what gives it one. Verkstead cannot tell that from a tray that is drawing the
icon, because the item registers on the bus either way, so there is no message
it could honestly give you. What is lost is the icon rather than anything the
icon was the only way to: the workbench is in the app's own window rather than
in a browser tab to be found again, **View Logs** is on the **Desktop** section
of the settings page as well as on the menu, and running the file again brings
the window forward rather than starting a second Verkstead — which is the way
back to a window that was closed. The extension is what gets the icon, and there
is nothing to reinstall or reconfigure here once it is on.

**And an app that started before the panel never gets one either.** A tray icon
is registered with a watcher on the session bus, and Chromium registers once: a
panel arriving afterwards — a desktop still coming up, a panel restarted — finds
nothing to draw, and that run stays iconless however long it lasts. **Show tray
icon**, turned off and then on again, is the way back: it takes that tray down
and raises another, which registers with the watcher that is there now and puts
the icon on the panel. Nothing else about the run is touched by the trip, which
is what makes the switch a way back as well as a way out.

**Three things stay the machine's**, and a bundle is the wrong place for any of
them.

**Sessions need bubblewrap**, and it cannot ride inside: an AppImage is mounted
`nosuid` and its files sit at a path made for one run, so a copy carried in the
bundle would be denied the privilege bwrap needs however it was granted. The
NixOS module puts it on the service's path; a desktop elsewhere wants the
distribution's `bubblewrap` package installed.

**The C library is the host's**, because a process holding two of them has two
of everything a C library keeps. Nothing in this file was compiled against your
distribution and the CLI inside it is linked statically, so the floor is
Electron's own rather than anything a build of ours chose: glibc 2.25, which the
release leg reads back off the artifact instead of taking on trust. Ubuntu
18.04, Debian 10 and RHEL 8 are all above it, so a distribution still taking
updates will load it; one older than those says `GLIBC_2.25 not found` and
nothing friendlier.

**And FUSE, because an AppImage mounts itself.** It wants a `/dev/fuse` to open
and the `fusermount3` helper to open it with, and nothing else: the runtime
packed into this file carries its own squashfuse, so there is no libfuse for
anybody to install — the library current distributions stopped shipping is not
one it asks for. Every desktop install has both; a minimal or hardened one may
not, and without them the file says so — "Cannot mount AppImage, please check
your FUSE setup" — with `--appimage-extract-and-run` the way past it for a
machine you cannot change.

**A session's account is a Built Root, not your account.** It is made fresh
under `homes/<id>` in the Data Directory as each session starts, and bound at
`~/.claude`, `~/.codex` or `~/.grok`, or at OpenCode's two directories, in the
empty HOME bubblewrap makes. Only an allowlist of your account is bound into
it, read-write: the login file, so a login lands in your account, and the
memory store, so memory and transcripts do too — for Claude this Repo's and
this Worktree's entries under `projects/`, for Codex `sessions/` and
`memories/`, for Grok Build `sessions/` and `memory/`, and for OpenCode the
whole data directory. The one exception is a Grok Build login, which is copied
in and merged back as the session ends, because grok saves it by renaming a
file over it and a bind refuses that. The configuration file is one Verkstead
writes, carrying only how your account reaches its model, and for Claude
`~/.claude.json` is a copy of yours, merged back as the session ends. None of
your plugins, hooks, skills, rules, MCP servers, global instructions, history
or other repositories' transcripts are there.

**What takes your global instructions' place is the settings page.** Its
**Instructions** section is one text for the whole installation — not one per
Profile — and it is written into each Built Root as the file that harness
reads: `.claude/CLAUDE.md`, `.codex/AGENTS.md`, `.grok/AGENTS.md`, or
`AGENTS.md` in OpenCode's config directory. It goes in verbatim, with no
heading over it and no line saying where it came from, and it is read as the
root is built — so a change there reaches the next session and a session
already running keeps what it started with. Leave the box empty and no such
file is written at all. Your repository's own `CLAUDE.md` or `AGENTS.md` is in
the Worktree and is read exactly as it always was, under this one.

**The memory switch on a Profile says whether its store is shared.** It is on
by default. Turn it off on the Profile form and a session starts with an empty
store of its own inside the Built Root instead: fresh memory, none of your
transcripts, and nothing it remembers written into your account. Its own
transcript is still found and still reaches the Timeline. The same holds on a
Mac and on Windows.

### The desktop app, on a Mac

`Verkstead-universal.dmg` holds `Verkstead.app`: the app, the server it starts
and the viewer the two of them draw between them, with Electron's own browser
runtime beside them — so a Mac with none of that installed needs nothing else to
put the workbench on the screen. Universal, as the bundle before it was: the
Apple silicon build and the Intel one are both in the download, so there is one
file and no architecture to choose between. macOS 12 is the oldest it will start
on, which is the floor Electron's own runtime writes into the bundle. Open the
image and drag Verkstead into the Applications folder beside it in the window,
which is the whole of the install.

**The first launch is then refused, and that is expected.** The app is unsigned
— there is no Developer ID behind it, which is
[ADR-0020](adr/0020-electron-desktop.md)'s decision rather than an oversight.
What it carries is an ad-hoc signature, because an Apple silicon Mac will not
execute a binary with no signature at all, and that buys nothing here: a
signature with nobody behind it is one of the things Gatekeeper exists to
refuse. So it will not open an app that arrived over the internet just because
somebody double-clicked it. What it says is that macOS "could not verify"
Verkstead "is free of malware", in a dialog with no way past on it. There is a
way past, and it is three steps — the first of them being the launch that fails,
because the refusal is what puts Verkstead in the list the second step reads:

1. Double-click **Verkstead** in Applications, and click **Done** on the
   refusal.
2. Open **System Settings → Privacy & Security** and scroll down to
   **Security**. `"Verkstead" was blocked to protect your Mac` is there with an
   **Open Anyway** button beside it: click it, and authenticate.
3. macOS asks once more, in a dialog that this time has an **Open Anyway** on
   it. Click that, and the app starts.

Once, rather than at every launch: what was approved is that copy of the app,
and starting it afterwards — by hand, or from Launch on Startup — is ordinary.
Replacing it with a newer download is a different copy and wants the same three
steps again.

**What is on the screen after that is a window**, with a tile in the Dock and an
icon in the menu bar. The window is the workbench itself, loaded off
`127.0.0.1:8422` with nothing about the viewer changed to draw inside it, and
closing it leaves Verkstead running: the application stays in the Dock and a
click on its tile is the window back. `Cmd+Q` is the other ending, and it quits
at once — no warning, no question — taking the server and every session with it.
The strip at the top of the screen is the app's own — **Verkstead**, **Edit**,
**View** and **Window** under the Apple menu — and the menu on the icon in the
menu bar is three items: **Open** brings the window forward, **View Logs** opens
the file under `~/Library/Logs/Verkstead` that this run's logging goes to when
there is no terminal to print it in, and **Quit** stops the app and the server
with it.

**The window has no title bar**, as on Linux. The workbench's own heads are the
top of it, and what stands where a title bar would have been is your Mac's
traffic lights, inset from the left edge and in the head's first row — left of
the wordmark, which is where the sidebar's head leaves room for them. They
travel down the window with the head when the text size grows, so nothing in a
head is ever under them, and full screen is where macOS withdraws them and the
head takes that room back. What moves the window is any pane head, and a
double-click on one maximises it.

**There is nothing to learn about starting it.** The app takes no flags at all;
what it reads instead is the server's own environment, so `VERKSTEAD_DATA_DIR`
moves the Data Directory off `~/Library/Application Support/Verkstead` exactly
as it does for a `verkstead serve` run by hand. Everything else about this
machine is set rather than typed, on the **Desktop** section of the settings
page: **Show menu bar icon**, which starts on, **Launch on Startup**, and **View
Logs** beside them for the desktop where the icon is switched off. There is no
choice about closing the window here, that being the platform's own answer
rather than a position anybody picks.

**The window it opens is logged in.** Every page of the workbench answers 401
without the **Workbench Key**, the secret Verkstead keeps in its Data Directory
where no session can reach it — and the app reads that file itself, before there
is a server to ask one of, so the window comes up on the address with the key on
the end of it. There is nothing to keep anywhere and nothing to type: **Open**
and the Dock tile are both that same window brought forward rather than a login
handed over again. The app's own startup line names the address and no key,
because **View Logs** opens a file on your desk and a workbench key written into
it would be a login for anybody reading over your shoulder.

**Launch on Startup is the login item macOS keeps for itself**, the one listed
under **Login Items** in System Settings, rather than a launch agent written
into your home directory. The box is that list read: ticking it registers
Verkstead there, unticking it takes the registration away, and there is no copy
of the answer anywhere for the two to disagree about. A login start comes up
with no window on the screen while there is an icon in the menu bar to reach it
by, and with a window where that icon is switched off. **A Mac can also hold a
registration that is there and will not start**: macOS can report Verkstead's
login item as waiting on your approval, which is registered and inert, and
nothing Verkstead can call puts it back — so the box is greyed, and what it names
underneath is what to do about it. Verkstead is an **Application** row under
**Open at Login** in that pane rather than one of the switches beside it, so the
control on that row is the minus button: remove it there, and the box here
registers a fresh one. **And an upgrade
from the old menu bar app takes its registration over once**: that app wrote a
launch agent at `~/Library/LaunchAgents/net.tobico.Verkstead.plist` by hand,
which the login item list knows nothing about, so the first launch of this one
reads whether it said start at login, carries that into the registration and
removes the file. Once, at a launch, and nothing you are asked about: it is this
app's own registration under an older name rather than a second setting.

**What is inside is the app and the released `verkstead` beside it**, rather
than one binary whose entry point supplies a verb. The CLI sits in a directory
of its own under the app's resources, and it is the very build a Release
publishes — the two Mac binaries the same run's CLI leg made, downloaded into
the package and joined with `lipo` into one universal executable rather than
compiled a second time ([ADR-0020](adr/0020-electron-desktop.md)). It is what a
session started here is handed to ask with, so the two halves of an ask are one
build.

**Answering from your phone is the settings page's Remote access section**, as
it is on Linux, and Tailscale itself is the Mac's own. What differs is the
password dialog behind the serve checkbox: serving to a tailnet is refused for a
process that is neither root nor the tailnet's operator, and here the press is
put to you through `osascript`'s *with administrator privileges* — the Mac's own
authentication prompt.

**Sessions run on a Mac**, and what one may reach is the same description as on
Linux rendered over Apple's sandbox instead of bubblewrap: the Conversation's
Worktree, the Repo's git directory and the handoff directory writable, each
Companion Repo at the mode it was set to, the Sandbox Configuration's entries,
the Build Cache with the machine's one `sccache` behind it, a HOME of the
session's own with a Built Root made out of the Agent Profile's account inside
it, the Skills and the `verkstead` a session asks with read-only, the system
read-only, `/tmp`, the network whole and unfiltered, and nothing else of the
machine.

**And `~/.local/bin` among what it reaches**, which is the one thing a Mac
gives a session that your own `PATH` need not have named. That is where
Anthropic's installer puts `claude`, and an app you started from the Dock has
launchd's `PATH` — four system directories, and no line of your shell profile
in it. So on a Mac that directory is on every session's `PATH` whether or not
the server was started with it, read-only inside like every other install, and
ahead of Apple's own `/usr/bin` along with Homebrew's two prefixes and
`/usr/local/bin`: a session started from the Dock finds the same `git` as one
started from a terminal. It is Apple's system directories those lead and
nothing else you wrote — anything your own `PATH` puts in front of `/usr/bin`,
a version manager's shims or a `~/bin` of your own, a session still finds
first, in the order you wrote it. On Linux what your `PATH` names is still the
whole of it.

**`/tmp` is the one place a Mac session reaches more than a Linux one**, and
the one thing on that list that is not the same on both. On Linux it is a
filesystem of the session's own: it holds nothing of the machine's, and it goes
when the session does. A policy has nothing like that to offer, so on a Mac it
is your real `/tmp` — a session can read whatever else on the machine left
something there, and what it writes stays behind for whoever looks. That is
deliberate rather than an oversight: giving a session a temporary directory of
its own would mean refusing every tool that reaches for the literal `/tmp`,
which is most of them. Nothing of Verkstead's is kept there — the handoff
document a grilling writes goes under the session's own HOME on a Mac, so two
Conversations running at once are not writing to one path.

**What differs is that the boundary refuses rather than hides**, and it is worth
knowing which of the two you have. A session on Linux is in a namespace your
home directory was never in; a session on a Mac is looking at a machine that has
one, and is refused every byte of it. What it can still read is the metadata: a
path it may not open answers `stat` and then refuses to open, because that is
what a Mac looks like from inside a policy and a rule per path to pretend
otherwise would buy nothing. And what a mount makes out of nothing is made for
real instead: the session's HOME, the Built Root in it, and the directory
holding the Skills and the `verkstead` binary are all really there under the
Data Directory, and what keeps one Conversation out of another's is the policy
rather than the absence. The Built Root reaches the account through symbolic
links: to the login file, and, with the memory switch on, to the memory store —
this Repo's two entries under `projects/` for Claude, and the directories the
Linux section names for the other three. Claude and Grok Build save their login
by writing a new file and renaming it over the old one, which replaces the link
rather than writing through it. So a login changed
inside is written back over the account's own as the session ends, and the
link is made fresh for the session after.

**And here too, none of your global instructions are in the root** — nor your
plugins, hooks, skills, rules or MCP servers — with the settings page's
**Instructions** section standing in their place: one text for the whole
installation, written into the root as the file that harness reads, the same
four files the Linux section names. It is read as the root is built, so a
change there reaches the next session and a running one keeps what it started
with, and an empty box writes no file at all.

**Nothing outlives the app.** **Quit** off the menu bar icon is a stop where it
stands, as it is on Linux, and so are `Cmd+Q` and the process being killed
outright: every session and the compile server go with it either way. Linux has
that from bubblewrap's `--die-with-parent`; a Mac has no such flag, so Verkstead
starts a keeper beside each sandbox whose whole job is to end it once the server
is gone.

**Two things stay the machine's**, where three do on Linux.

**The tools a session runs**, because the bundle is the server and the viewer
and not a toolchain. `git`, `node`, `cargo` and whatever else an agent reaches
for are the Mac's own: Apple's under `/usr/bin`, where `git` arrives with the
Xcode Command Line Tools; Homebrew's under `/opt/homebrew`; and nix's under
`/run/current-system` where the Mac is running nix-darwin. All three are on a
session's `PATH` inside and readable through the boundary, and a Mac with none
of them installed has sessions that can run a shell and not much else.

**And the sandbox itself**, which is `/usr/bin/sandbox-exec`: on every Mac,
nothing to install, and deprecated by Apple with no replacement an unsigned app
can use — the supported way to sandbox is an entitlement on a signed bundle,
applied to the app itself rather than to a child it spawns. ADR-0012 takes that
with open eyes, and it is the one thing here that could stop working without
anybody touching Verkstead: the day the command goes, Mac sessions go with it
until something replaces them.

### The desktop app, on Windows

`Verkstead-x86_64.msi` is the download, and it is an installer rather than a
file to keep wherever you like: it holds the app, the server it starts and the
viewer the two of them draw between them, with Electron's own browser runtime
beside them, and what an install adds is the two doors Windows has — an entry
in the Start menu, and a directory on your `PATH`. x86_64 only, which is every
Intel and AMD machine, and an arm64 one runs it under the emulation Windows
does for exactly this.

**Opening it is stopped the first time, and that is expected.** The package is
unsigned — there is no code-signing certificate behind it, which is
[ADR-0020](adr/0020-electron-desktop.md)'s decision rather than an oversight —
and Windows marks a file that arrived from the internet, so SmartScreen puts a
blue **Windows protected your PC** window in front of it with a **Don't run**
button and nothing else that looks like a way on. There is a way on, and it is
two clicks:

1. Click **More info**, which is the line under the message and the whole of
   what is hidden here.
2. It names the file and says *Unknown publisher*, and a **Run anyway** button
   appears at the bottom. Click that, and the install runs.

Once, at the install rather than at every launch: what is started afterwards is
the Start-menu entry, which Windows put there itself and says nothing about. A
newer download is a different file and wants the same two clicks. The other way
round is to take the mark off before opening it instead: right-click the msi,
**Properties**, and tick **Unblock** at the bottom of the **General** tab.

**It installs into your own profile, and asks nobody for anything.** The app is
unsigned, so an installer asking for administrator would be an unsigned program
asking for the machine, and elevation buys a downloader nothing they wanted.
There is one elevated step on this platform and it is below, after the install
rather than inside it: one you take once, having already decided to trust this,
rather than a package you hand the machine to before you have seen it.
Everything therefore lands in the profile: the app under
`%LOCALAPPDATA%\Programs\Verkstead`, a **Verkstead** entry in your own Start
menu, and the uninstall entry in **Installed apps** beside everything else you
installed. There is nothing to choose on the way through — a progress window,
and then Verkstead itself, which the installer starts as it finishes. A newer
msi replaces the copy that is there rather than standing beside it.

**One thing about that install may land in the wrong list.** The row in
**Installed apps** is Windows Installer's own to write and the hive it writes
it in is not something a package chooses: the msi this one replaces had it
under the machine's `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall`
rather than under your `HKCU`, so **Installed apps** offered it to everybody
who signs in to this machine instead of only to you, and this package may yet
do the same. What that row cannot say is where the install went. That is
written under your own `HKCU\Software\Microsoft\Installer\Products` and nowhere
machine-wide, and the files, the Start-menu entry and the `PATH` entry are all
in your profile — so what a row in the wrong list costs is an entry somebody
else can see rather than an install that touched the machine. Uninstalling from
**Installed apps** works either way.

**And a directory inside the install goes on your `PATH`**, which is the half
of this download that is not the window at all: `verkstead ask`, `verkstead
guide` and the rest work in a terminal opened *after* the install. What goes on
it is `resources\cli` under `%LOCALAPPDATA%\Programs\Verkstead` — the CLI's own
directory rather than the install root, because the root is where the launcher
`Verkstead.exe` stands and Windows resolves a `PATH` lookup without regard to
case, so a root on it would answer `verkstead guide` with a window instead of
the Guide. A terminal that was already open never read the entry — closing it
and opening another is the whole of the fix — and the uninstall takes the entry
away with the files.

**What is on the screen once Verkstead is opened from the Start menu is a
window**, with a button on the taskbar and an icon in the notification area.
The window is the workbench itself, loaded off `127.0.0.1:8422` with nothing
about the viewer changed to draw inside it, and the menu on the icon is three
items: **Open** brings the window back, and is what a click on the icon does;
**View Logs** opens the file under `%LOCALAPPDATA%\Verkstead` that this run's
logging goes to when there is no console to print it in; and **Quit** stops the
app and the server with it. Closing the window leaves Verkstead running with
that icon to bring it back, which is what **When the window is closed** says
until you say otherwise. **Windows hides an icon it has not seen before**, in
the flyout the `^` on the taskbar opens — dragging it out of there onto the
taskbar is what pins it, and until you do, the app is running with its icon one
click further away than this describes.

**The window has no title bar**, as on Linux and a Mac. The workbench's own
heads are the top of it, and what stands where a title bar would have been is
Windows' own minimise, maximise and close, inset at the top-right corner and
drawn on the paper a pane head is drawn on with their symbols in the head's
ink. A machine in the dark scheme gets the dark paper, and a flip while the app
is running recolours them in that run rather than at the next launch. They are
the platform's own rather than something the page drew, which is what keeps
**snap layouts** on a hover of the maximise button. Nothing of the page's is
ever under them: the head at each edge of the window is padded by what the
controls took, read again whenever the window changes shape. The gestures
either side of them are the app's: a double-click on any pane head maximises
the window and a second one restores it, and `Ctrl+M` puts it away. The menu
bar is hidden and Alt is what brings it down, with that same **Minimize** under
**Window** where a keystroke was not what you wanted.

**There is nothing to learn about starting it.** The app takes one flag,
`--hidden`, and its own startup registration is what writes that rather than
anybody typing it; what it reads instead is the server's own environment, so
`VERKSTEAD_DATA_DIR` moves the Data Directory off `%APPDATA%\Verkstead` exactly
as it does for a `verkstead serve` run by hand. Everything else about this
machine is set rather than typed, on the **Desktop** section of the settings
page: **Launch on Startup**, **When the window is closed** — keep running in
the tray, ask first, or quit — and **Show tray icon**, with **View Logs**
beside them for the machine where that icon is switched off.

**The window it opens is logged in.** Every page of the workbench answers 401
without the **Workbench Key**, the secret Verkstead keeps in its Data Directory
where no session can reach it — and the app reads that file itself, before
there is a server to ask one of, so the window comes up on the address with the
key on the end of it. There is nothing to keep anywhere and nothing to type:
**Open** is that same window brought forward rather than a login handed over
again. The app's own startup line names the address and no key, because **View
Logs** opens a file under your own `%LOCALAPPDATA%` that anybody at the screen
can read.

**Launch on Startup is a value under the Run key**, `Verkstead` under
`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, naming the app where it
is installed with `--hidden` after it. The box is that value read: ticking it
writes it, unticking it takes it away, and there is no copy of the answer
anywhere for the two to disagree about. A login start comes up with no window
on the screen while there is an icon in the notification area to reach it by,
and with a window where that icon is switched off. **Windows' own second
opinion is read as well**: the **Startup apps** tab in Task Manager keeps its
switch in a key of its own that the value knows nothing about, so a Verkstead
switched off there reads as off here — Verkstead agrees with you rather than
putting back what you took away — and ticking the box again is what turns it
on. **And an upgrade from the old tray app takes its registration over once**:
that app wrote a value of its own beside this one, named
`net.tobico.Verkstead` after the application rather than after the product, so
the first launch of this one reads whether it said start at login, carries that
into the registration and removes it. Once, at a launch, and nothing you are
asked about: it is this app's own registration under an older name rather than
a second setting.

**Answering from your phone is the settings page's Remote access section**, as
it is on the other two, and Tailscale itself is the machine's own. What differs
is the elevation prompt behind the serve checkbox: serving to a tailnet is
refused for a process that is not the tailnet's operator, and here the press
comes up as a **User Account Control** dialog.

**What is inside is the app and the released `verkstead` beside it**, rather
than one binary whose entry point supplies a verb. The CLI sits in a directory
of its own under the app's resources — `resources\cli`, which is the directory
the install put on your `PATH` — and it is the very build a Release publishes,
the Windows exe the same run's CLI leg made, downloaded into the package rather
than compiled a second time ([ADR-0020](adr/0020-electron-desktop.md)). It is
what a session started here is handed to ask with, so the two halves of an ask
are one build. It is an ordinary console program, which is what a terminal, a
session and a test all want of it; the Start-menu entry opens `Verkstead.exe`
at the install root instead, and that is what keeps a black window off the
screen when somebody clicks it.

**Then one elevated step, once — and the wizard takes it for you.** Sessions on
this machine run as a local account of Verkstead's own rather than as you —
which is what keeps an agent out of your Documents — and creating a local
account is an administrator's call. The setup wizard's first step is where that
happens: the sandbox row reads absent until there is an account, ticking it and
pressing **Next** runs the command below behind a **User Account Control**
prompt, and the row goes present without anything being restarted. That is the
one thing Verkstead has ever asked you to elevate for, and it asks once.

**By hand, where the wizard could not.** A Verkstead started from a terminal
rather than from its icon has no screen to put a UAC prompt on, and sends the
row to the wizard's last screen with this line on it instead. Right-click the
Start button, choose **Terminal (Admin)** or **Windows PowerShell (Admin)**,
answer the prompt, and run it there:

```console
$ verkstead session-account create
account   = vk-3f9c1a2b7e04 created
password  = written to C:\Users\you\AppData\Roaming\Verkstead\secrets.yaml
next      = this Verkstead's Windows sessions will run as vk-3f9c1a2b7e04
```

The name is short because a local account name may be twenty characters and no
more, and the digits after `vk-` come off the Data Directory — so two
Verksteads on one machine keep their accounts apart the way they keep their
pipes apart. The password is long, random and never typed by anybody: it lands
beside your other secrets under `%APPDATA%\Verkstead`, which is where the
server reads it from to start a session. Run the command a second time and it
says the account was already there rather than failing. Run it in an ordinary
terminal and it refuses with a line naming what it needs, which is this.

**Everything after it is unprivileged**: the server only ever *reads* the
account, and starting a session as it wants nothing of an administrator. Until
the command has been run there is no account for a session to be, and a session
is **refused** rather than started without a boundary — the log says which, the
way it does on a Linux machine with no `bwrap`.

**And `verkstead session-account remove`, elevated too, takes it back off**: the
account, its password out of the secrets file, and the `C:\Users` directory
Windows made for it the first time a session ran. Worth running before you
uninstall, because the msi knows about none of the three — a machine that has
run Verkstead and then had it taken off should look as it did. Nothing there is
a failure for not having been found, so it is also how a half-finished install
is tidied up.

**Sessions run on Windows**, and what one may reach is the same description as
on Linux and a Mac rendered over the **local account** the elevated command
above made. A session runs on a pseudoconsole Verkstead opens for it, as that
account rather than as you: the Conversation's Worktree, the Repo's git
directory and the handoff directory writable, each Companion Repo at the mode
it was set to, the Sandbox Configuration's entries, the Build Cache with the
shared `CARGO_HOME` inside it, a profile of the Conversation's own with a Built
Root made out of the Agent Profile's account inside it, the Skills and the
`verkstead` a session asks with read-only, Windows and Program Files
read-only, a temporary directory of the session's own, the network — and nothing else of the machine.
Not your Documents and not the rest of your profile.

**The boundary refuses rather than hides**, as a Mac's does and unlike Linux's:
your home directory is in plain sight from inside a session and every byte of
it is refused — to reads as much as to writes, an account being granted your
files no more than a stranger's is. `verkstead ask` goes through a **named
pipe** the server opens beside its socket: that was the only way in behind the
boundary this platform used to have, and it stays because it is the one
transport no firewall on the machine has to agree with.

**One account serves every Conversation**, which is the one place this
boundary is weaker than the other two platforms', and it is worth being plain
about: making a local account takes an administrator, so one per Conversation
would take an administrator per Conversation. A session can therefore reach
every *live* Conversation's Worktree — not a Closed one, whose entries came off
with it. What it cannot reach is your machine: your profile, your Documents,
and the checkouts your Repos were registered from — a Worktree being all a
session ever sees of a repository, here as on the other two platforms. That is
the boundary this exists for, and it is the one that is asserted rather than
assumed: the Windows suite attempts your own files from inside a session and
reads back the refusal.

**What has no equivalent on either other platform is that the boundary is
written on your own directories.** An account reaches what it has been granted
and nothing else, so there is nothing to mount and no policy to hand a process:
each real path the description names gets an access-control entry for that
account — a grant on the Worktree at the reach the description says; for the
account, a grant on the Built Root, one on each directory of the memory store
joined into it, and one on the login file itself, but none on the account
directory as a whole; and a step through each directory on the way to
any of those, so that a path can be resolved without its parent becoming
something to list. Two
things about those entries are worth knowing, because they are on directories of
yours rather than on anything of Verkstead's:

- **Closing takes them away.** A Conversation closing takes its entries off in
  the same breath as its Worktree: every one written for it comes off the
  directory it was written on. What that is read off is a record Verkstead
  writes under its own Data Directory before anything is granted — the account
  it is granting and every entry written — because a Closed Conversation has no
  Worktree left to work the list out from again.
- **A crash is swept up at the next startup.** A server that stopped between
  the two would leave entries on your directories, so the next one to start
  reads those records and takes back the entries of every Conversation that is
  Done or Closed or gone from the record altogether.

**A boundary that cannot be made refuses the session**, the way a missing
`bwrap` does on Linux: no account on the machine, no password beside the
secrets, or an entry that will not be written. There is no unsandboxed session
to fall back to, and the log says which of the three it was.

**The profile a session runs in is the Conversation's own**, under
`%APPDATA%\Verkstead\homes`, emptied and made again as each of that
Conversation's sessions starts — except while another session or terminal of it
is still running in there, which is given the profile as it stands rather than
having it deleted out from under it. `USERPROFILE` and `HOME` point at it, and
`APPDATA`, `LOCALAPPDATA`, `TEMP` and `TMP` point inside it — so what npm
caches, what a tool writes down and what either of them throws away lands there
rather than in your own profile.

**No account is joined into it whole.** The profile gets a Built Root — at
`.claude`, `.codex` or `.grok`, or OpenCode's two directories — and only an
allowlist of the account is joined into that. The login file is joined by a hard
link, so a session starts logged in. With the memory switch on, the memory store
is joined by directory junctions, so memory and transcripts land in the account:
for Claude this Repo's entry and this Worktree's entry under `projects/`, and
for the others the directories the Linux section names. There is no other
repository's transcripts, and none of your plugins, hooks, skills, rules, global
instructions or history. The root's configuration file is one Verkstead writes.
Beside a Claude root, `.claude.json` is a copy of yours with your MCP servers
taken out and this Repo marked as trusted.

**Your global instructions' place is taken by the settings page's
Instructions section**, as it is on the other two platforms: one text for the
whole installation, written into the root as the file that harness reads —
`.claude\CLAUDE.md`, `.codex\AGENTS.md`, `.grok\AGENTS.md`, or `AGENTS.md` in
OpenCode's config directory — verbatim and with nothing added to it. It is read
as the root is built, so a change reaches the next session and a running one
keeps what it started with, and an empty box writes no file. Your repository's
own `CLAUDE.md` or `AGENTS.md` is in the Worktree and is read under it, exactly
as it always was.

**A hard link wants one volume**, which is the one thing about this that can
refuse a session outright. Your account's files and the Data Directory have to
be on the same drive; where they are not, the session does not start and the
log says which two paths those are and which of them to move.

**What a session changed in its account is carried back as it ends.** A hard
link stops being one file the moment something saves over it by writing a
temporary file and renaming it into place, which is how Claude and Grok Build
save a login.
So a linked file the session replaced is written back over the account's own,
and the link is made fresh for the session after; one still the same file is
left alone. The copied `.claude.json` is merged rather than copied back: only
what the session changed reaches your file as it is by then, so what another
session or your own `claude` wrote in the meantime is kept, and your MCP servers
are never touched. A session that changed nothing leaves the file exactly as it
was.

**A Conversation Terminal opens on Windows PowerShell**, in the Worktree, on
the same pseudoconsole a session runs on. Not `pwsh`, even where you have
PowerShell 7: that on most machines is a Store execution alias living under
your own profile, which is per-user by construction, and the account a terminal
runs as is refused it. So the shell every Windows machine really has is the
answer rather than the fallback.

**The tools a session runs are the machine's**, as they are on a Mac: `git`,
`node`, `cargo` and whatever else an agent reaches for are found on the `PATH`
the server itself was started with, with Verkstead's own directory in front of
it — so an agent npm installed as a `claude.cmd` starts as readily as an
installer's `claude.exe`. Everything else there is what it is on the other two:
the Repos, the Briefs, the Question Sets, the Timeline, the pull requests.

**Rust builds share both halves here, as they do everywhere.** The Build Cache
is a directory of Verkstead's own — `%LOCALAPPDATA%\Verkstead\Cache` unless you
say otherwise — that a session reaches read-write with `CARGO_HOME` inside it,
so a crate is downloaded once for the machine rather than once per
Conversation. The other half, the compiled objects, wants an `sccache` on the
`PATH` the server was started from: with one there, every session's `rustc`
goes through the single Compile Server Verkstead runs, and a dependency is
compiled once for the machine too. The workbench's Language support page says
which of the two you have. A C/C++ Repo goes through the same server only where
CMake is told `-G Ninja`: the Visual Studio generator it picks by default
ignores the launcher variables (see [C/C++ through the Compile
Server](#cc-through-the-compile-server)).

**The toolchain a session builds with is the one you installed.** `rustup`'s
shims are on your `PATH` already, and the rustup home they resolve a toolchain
out of is reached read-only beside them — so `cargo build` inside a session
uses your default toolchain rather than one Verkstead brought. A machine
without rustup is a machine where a session finds whatever else is on the
`PATH`.

Out of a checkout instead — the same server, told `--data-dir .` so that
`verkstead.db` and the rest land in the checkout rather than in the platform
directory — is [development.md](development.md#quickstart).

## Languages

A language is a **descriptor**: data, in one grammar, saying what to call it,
what says a checkout builds it, and what variables a session is given. Seven
ship — Rust, C/C++, Go, Node, Python, .NET and the JVM — and there is nothing
special about any of them: each is an entry in a YAML file embedded in the
binary, written exactly the way you would write one. So the built-ins below are
both what Verkstead does and the worked examples of the grammar. What C/C++
covers is [its own section](#cc-through-the-compile-server), and what each of
the other five shares, tool by tool, is [at the end of this
section](#what-each-language-shares).

This is the whole of Rust's, as it ships:

```yaml
languages:
  rust:
    label: Rust
    detect:
      - Cargo.toml
    env:
      CARGO_HOME: "{cache}/cargo"
    capabilities:
      sccache:
        env:
          RUSTC_WRAPPER: "{sccache}"
          SCCACHE_DIR: "{cache}/sccache"
          SCCACHE_CACHE_SIZE: "{size}"
```

`label` is what the settings page calls it. `detect` is what says a Repo builds
it, looked for at the root of a checkout — **for the composer's warning and
nothing else**: the variables are every session's whatever the Repo holds,
because a manifest is often not at the root and a variable nothing reads costs
nothing. `env` is those variables. `capabilities` names behaviour the server
has built in, which a descriptor can *name* and cannot *describe*: `sccache` is
the **Compile Server**, and a capability's variables are set only on a machine
that can offer it — no sccache anywhere and a session gets the downloads above
and none of the three below. There is no key for a command to run, and there
will not be one: a settings file that started programs is a settings file whose
sandbox somebody then has to describe in YAML too.

Two more keys belong to the same entry and are not in the file above, because
what they say is your machine's rather than the release's: `enabled`, whether
sessions get this language at all, and `size`, how big its store may grow —
which is what `{size}` stands for. Absent, they are **on** and **30G**. They
are also the only two keys the settings page ever writes, so a save from the
**Language support** pane leaves everything else in an entry exactly as you
typed it.

**Your own entries merge over the built-ins, key by key.** What you write goes
under `languages:` in `config.yaml` in the data directory, and each key lands
in the built-in entry of that name. So changing one variable keeps every later
fix to the rest:

```yaml
# config.yaml — this machine's Build Cache is a spinning disk and its Worktrees
# are not, so the registry goes beside the Worktrees instead. Everything else
# about Rust — its manifest, its Compile Server and that server's own store —
# is still whatever this Verkstead ships.
languages:
  rust:
    env:
      CARGO_HOME: "{stores}/cargo"
```

and a variable set to `null` is taken **out**, which is how one of the
built-ins above is dropped without writing the rest of them again:

```yaml
languages:
  rust:
    env:
      CARGO_HOME: null
```

**A name Verkstead has never heard of is a language in its own right.** It is
on from the next session, with its variables in every Sandbox and its own box
on the settings page, and no release has to know about it:

```yaml
languages:
  gleam:
    label: Gleam
    detect:
      - gleam.toml
    env:
      HEX_HOME: "{cache}/hex"
```

**The placeholders are the things only the server knows**, and the two worth
choosing between are `{cache}` and `{stores}`. `{cache}` is the **Build Cache**
— `--build-cache-dir`, else `/var/cache/verkstead` on the packaged unit and the
platform's own cache directory otherwise — and it is where a store goes that
nothing has to share a filesystem with. Rust's two are both there: a registry
and a pile of compiled objects are *read*, wherever they are. `{stores}` is a
directory beside the **Worktrees**, under the data directory, and it is for a
store that *does* care: pnpm, deno, bun and uv hardlink packages out of theirs
into the project rather than copying them, and fall back to copying the lot
where the store and the project are on different filesystems. The Build Cache
is free to be a second disk — the flag may name one outright, and the packaged
unit's `CacheDirectory` and `StateDirectory` are two mounts a sysadmin
separates as a matter of course — so a store that has to be next to the
checkout says so. Four of the built-in variables do, and an entry of your own
says it the same way — conda hardlinks a package out of its cache into an
environment and copies it where it cannot, so its cache belongs there too:

```yaml
languages:
  conda:
    label: conda
    detect:
      - environment.yml
    env:
      CONDA_PKGS_DIRS: "{stores}/conda"
```

**On Linux they all copy anyway today**, which is worth knowing before you
count on the space a hardlink saves: a Worktree and this directory are two
separate bind mounts inside a session's sandbox, and a hardlink does not cross
two mounts however few disks are underneath them. The store still does its
work — a copy out of it is a download that did not happen — and pnpm and uv say
in as many words that they fell back, where deno and bun do it without a word.

Write `/` after a placeholder whatever platform you are on. A descriptor is one
file read on three, so the grammar has one separator, and a Windows session is
handed the path its own tools would have composed.

The other two are not a choice. `{size}` is that entry's own `size` key, and
`{sccache}` is where a session reaches the sccache this server found — it only
means anything inside the `sccache` capability, which is what says there is one
at all. A placeholder's directory is made, and opened to a session, only where
a loaded descriptor names it: four of the shipped variables name `{stores}` —
pnpm's store, deno's cache, bun's and uv's — so every session with Node or
Python switched on is opened onto that directory, and an install with both of
them off is opened onto none of it.

**What an entry that will not load costs you is the entry, and nothing else.**
Two ways one fails. Naming a variable the Sandbox sets itself — `PATH`, `HOME`,
`VERKSTEAD_SERVER`, `RUSTUP_HOME`, and the Windows names for a profile that the
two Unixes have no equivalent of, refused on every platform so that a file
which loaded on a Mac cannot break the same install on a Windows box — is
refused by name, because a descriptor that could rewrite those is one that
could take a session's `verkstead` away from it, or its agent's login. And an
entry nothing can parse is refused the same way — **a key this grammar does not
have included**, which is what stops `detct:` being a manifest list that
silently did nothing. Either way that language falls
back to the descriptor Verkstead ships, which is the cache you already had;
every other language loads; the server comes up. A language with no built-in
behind it — your Gleam, with a typo in it — goes **off** rather than on at
nothing.

The **Language support** pane is where you find out. That language's box is
drawn with its controls off and a sentence saying why: the reason, naming the
variable where a variable is what was refused, and which of the two became of
it. Nothing of what you typed is thrown away — a save from that page writes the
whole of `config.yaml` and puts your entry back exactly as it was — so the fix
is in the file, and the next session reads it. Settings are read at every
session spawn: nothing restarts.

### C/C++ through the Compile Server

C/C++ is the second descriptor naming the `sccache` capability, and it names
the same one Rust's does: one **Compile Server** for the machine, one store and
one size, so there is no second server and no size field of its own on the
settings page. Its box turns C and C++ compiles through that server on and off.
This is the whole of it:

```yaml
languages:
  cpp:
    label: C/C++
    detect:
      - CMakeLists.txt
      - native/CMakeLists.txt
      - cpp/CMakeLists.txt
      - meson.build
    capabilities:
      sccache:
        env:
          CMAKE_C_COMPILER_LAUNCHER: "{sccache}"
          CMAKE_CXX_COMPILER_LAUNCHER: "{sccache}"
```

C++ has no one manifest, so `detect` is the build files a checkout ordinarily
has one of — and, as for every language, it drives the composer's warning and
nothing else. With no sccache on the server the capability is left out whole,
and a session gets neither launcher.

**What is covered is CMake**, 3.17 and later, with the **Makefile or Ninja**
generator. CMake reads the two launcher variables out of the environment when a
build directory is **first configured**, and caches what it read — so a build
directory configured before C/C++ was switched on keeps compiling without the
server until it is configured afresh (delete it, or its `CMakeCache.txt`).
**Meson** is covered without Verkstead doing anything: it finds an `sccache` on
the `PATH` for itself, and a session's `PATH` has the server's.

**What is not covered:**

- **Plain Makefiles and Bazel.** Neither reads anything a descriptor could set
  without reaching past C++ projects, so they compile uncached.
- **CMake's Visual Studio and Xcode generators**, which ignore the launcher
  variables. Visual Studio is CMake's default on Windows, so **a Windows Repo
  is cached only where it configures with Ninja** (`-G Ninja`).
- **MSVC debug information in `/Zi` form**, which sccache cannot cache; `/Z7`
  it can. That is a project's compile flags, and Verkstead leaves them alone.

**Why `CC` and `CXX` are left alone.** Setting them would have reached every
Makefile too — and every build that compiles C and is not a C++ project at all:
a Rust crate's C build script, a Python or Node native extension, Go with cgo.
Beside a launcher, they can wrap one compile in sccache twice. The launcher
variables are read by CMake and nothing else, so they change nothing that is not
a CMake project. This was asked for and withdrawn once its reach was laid out
([ADR 0021](adr/0021-language-descriptors.md)); a Repo that wants its own
Makefile cached can still name `sccache` as its compiler itself.

**A second Conversation hits the first one's objects** because the Compile
Server is told every Worktree as a base directory. sccache hashes a C or C++
compile with its absolute paths, and every Conversation's Worktree is a path of
its own, so without that every Conversation would compile everything again.
Three things are worth knowing about it:

- It needs **sccache 0.14.0 or later**, the first to honour
  `SCCACHE_BASEDIRS`. An older one ignores the variable: nothing fails, and a
  C/C++ build in a second Conversation simply misses. Rust is unaffected
  either way, its dependencies being compiled out of one `CARGO_HOME` at one
  path from every Worktree.
- The server reads the list **once, as it starts**, and is restarted to take in
  a new Worktree only while no session is running — restarting it under a
  build would fail that build. So on a machine that is never quiet a new
  Conversation's builds miss until it is.
- An object served from another Conversation's compile carries **that
  Worktree's path in its debug information**. A debugger opened on it looks for
  the source there.

**If a compile fails rather than missing the cache**, it is one of two limits of
the server's own Sandbox, which holds the Worktrees, the Build Cache and the
system, and nothing else:

- **A build directory outside the Worktree.** The server writes each object
  where the compile names it, and a directory it cannot reach — the session's
  own `/tmp`, say — is an error, not a miss. Configure the build directory
  inside the Worktree (`cmake -B build`). The same holds for Rust's `target/`
  and a `CARGO_TARGET_DIR` pointing elsewhere.
- **A compiler reached only through the Conversation's own binds.** The server
  runs the compiler, and it sees none of a Conversation's **Sandbox
  Configuration**: a toolchain bound into one Conversation is one the server
  cannot run. A compiler the machine itself has installed — what every Sandbox
  reaches, a dev shell's under `/nix` included — or one inside the Worktree is
  fine.

### What each language shares

Rust's descriptor is above, and C/C++'s is the section before this one. Four
of the other five are the **package stores**, and the JVM, whose builds share
more than downloads, is [at the end](#the-jvm). In all five, every
tool that installs from one ecosystem's registry is in that ecosystem's entry
rather than one of its own, so one box on the **Language support** pane turns
the lot of them on or off, and a session gets every variable of a language that
is on whatever its checkout holds.

Each row below is one variable the built-in sets, where its directory goes, and
what moves there. `{cache}` is the **Build Cache** and `{stores}` is the
directory beside the **Worktrees** — a store is under the second one where its
tool hardlinks a package out of it into the project.

**Go**, detected by `go.mod`:

| Variable | Where | What moves there |
| --- | --- | --- |
| `GOMODCACHE` | `{cache}` | the modules the `go` command downloaded |
| `GOCACHE` | `{cache}` | **the compiled half**: what a build compiled, which for Go is a directory and nothing else |

Go is the one ecosystem here whose compiled output is shared as well, and it is
the easy kind: where Rust's second half wants an `sccache` running, Go's is a
directory two sessions both write.

**Node**, detected by `package.json` — npm, pnpm, both yarns, deno and bun,
with no variable name shared between them:

| Variable | Where | What moves there |
| --- | --- | --- |
| `NPM_CONFIG_CACHE` | `{cache}` | npm's `_cacache`: the packages it downloaded, and what `npm --offline` installs out of |
| `PNPM_CONFIG_STORE_DIR` | `{stores}` | pnpm's content-addressable store, the one it hardlinks packages out of. Not `PNPM_HOME`, which is where pnpm puts global binaries and holds no packages at all |
| `PNPM_CONFIG_CACHE_DIR` | `{cache}` | the registry metadata beside that store, which an offline install needs as much as the packages |
| `YARN_CACHE_FOLDER` | `{cache}` | Yarn Classic's cache — yarn 1.x, still what `yarn` is on most machines |
| `YARN_GLOBAL_FOLDER` | `{cache}` | Yarn Berry's, which is yarn 2 and up. Berry keeps its cache here rather than in the folder above, so the two yarns are two directories |
| `DENO_DIR` | `{stores}` | deno's whole cache: remote modules, npm packages, what it emitted — and its origin storage, below |
| `BUN_INSTALL_CACHE_DIR` | `{stores}` | bun's package cache. Not `BUN_INSTALL`, which is bun's install root and whose `bin` is on a session's `PATH` |

No compiled store among the six: the one piece of built output any of them
keeps is what deno emitted, which rides along inside `DENO_DIR` rather than
being a second directory anybody chose to share. Everything else a build makes
is in the project.

**Python**, detected by `pyproject.toml` or `requirements.txt` — pip, uv,
poetry and pipenv:

| Variable | Where | What moves there |
| --- | --- | --- |
| `PIP_CACHE_DIR` | `{cache}` | pip's downloaded responses and the wheels it built out of an sdist |
| `UV_CACHE_DIR` | `{stores}` | uv's: the index responses, the downloads, and the unpacked wheels it links into an environment |
| `POETRY_CACHE_DIR` | `{cache}` | poetry's downloaded distributions and the release information it resolved against — an install offline needs both |
| `PIPENV_CACHE_DIR` | `{cache}` | pipenv's, which **is** the pip cache a pipenv install uses: pipenv builds pip's environment itself and passes none of the session's other `PIP_` variables through, so the row above does nothing for it |

And two that are not directories at all: `POETRY_VIRTUALENVS_IN_PROJECT` and
`PIPENV_VENV_IN_PROJECT`, both on. **Only downloads are shared, never a virtual
environment.** Poetry's would otherwise go under the cache directory above,
which is to say into the shared store, and a virtual environment holds absolute
paths — one built in another Worktree is broken in this one. So every `.venv`
is in the Worktree that made it, where it also outlives the session. No
compiled half here either.

**.NET**, and the one tool every .NET machine installs through, NuGet:

| Variable | Where | What moves there |
| --- | --- | --- |
| `NUGET_PACKAGES` | `{cache}` | the global packages folder: what restore downloaded and unpacked, and what a build then compiles against where it lies |
| `NUGET_HTTP_CACHE_PATH` | `{cache}` | the responses behind it — the service index, the version lists and the `.nupkg` as it came off the wire |
| `NUGET_SCRATCH` | `{cache}` | NuGet's temp directory, which holds no downloads. It is here because the lock restore takes before it extracts a package is a *file* in it, and two sessions that cannot see one lock extract over each other |

This is the one entry with an empty `detect`, and on purpose: a .NET project is
a `*.csproj`, a `*.fsproj` or a `*.sln`, and `detect` matches literal filenames
rather than globs. What that costs is the composer's warning on a .NET Repo and
nothing else — the variables are every session's either way. And there is no
compiled half: what a build leaves is `obj/` and `bin/` in the project.

**A shared store is a writable store, which means one session can plant a
package another installs.** A Conversation that writes into pnpm's store or
NuGet's packages folder is writing where the next one reads, and nothing here
checks what it put there. That is accepted rather than mitigated: it has been
true of Rust's registry since there was a Build Cache, the machine is one
person's, and the alternative is every Conversation downloading the internet
again. What is *not* shared is anything that is a session's own — a registry
login lives beside these directories rather than in them, and none of the
variables above moves the directory a login is in.

**`DENO_DIR` is the one that holds more than downloads.** deno keeps its origin
storage there: `localStorage` for a program run with a `--location`, and the
database `Deno.openKv()` opens where it was given no path. So two Conversations
running one Repo's program see one `localStorage` between them. That is a
program's own state rather than a secret, and it is the same bargain as the
store — but it is a bargain, so it is written down here.

**A Repo's own configuration does not win over these.** Every one of these
tools with a config key for its store puts the environment above that file, so
a session's variable beats an `.npmrc`, a `pnpm-workspace.yaml`, a `.yarnrc` or
a `bunfig.toml`, and a Repo that must have a store of its own passes
`--cache`, `--store-dir` or `--cache-folder` on the command line where it
installs. Nothing a descriptor can do changes that: the grammar sets variables,
and there is no rung below the environment to set one on.

**Two Conversations installing with Yarn Classic at the same moment may cost one
of them its install.** Classic is the one tool of the eleven whose cache is not
safe for two writers: it makes a cache entry at that entry's final name and
fills it afterwards, so an install arriving in between finds the directory,
takes it for complete, and fails on a file that is not written yet — a yarn
error naming a `.yarn-tarball.tgz` under the cache. **Nothing is damaged**: the
store is left usable, and the install is right the second time. The cache stays
shared because that is the trade — one download for the machine against a
collision you re-run — and it is written here so it is recognised rather than
debugged. The other ten tools, Yarn Berry included, are safe for two at once.

**The one Repo that loses something it cannot ask back for is a Yarn Berry Repo
doing zero-installs.** A Repo that writes `enableGlobalCache: false` is asking
Berry to keep its cache in the checkout — vendored, committed, installed from
without a registry. What that switch actually does is send Berry from
`globalFolder/cache` back to `cacheFolder`, and `cacheFolder` is
`YARN_CACHE_FOLDER`, which is the variable the session set for Yarn Classic. So
that Repo lands in a shared directory rather than its own, and **Berry has no
command-line flag that moves the cache back** — the paragraph above does not
apply to it, there being nothing to pass. What it keeps is an install that works
out of a store; what it gives up is the project-local cache it would have had
without Verkstead. `YARN_ENABLE_GLOBAL_CACHE` would settle it either way and is
deliberately not set: moving a store is what a descriptor is for, and overriding
a Repo's policy switch is not. A Repo that has to have the vendored cache turns
**Node** off on the settings page.

### The JVM

**One `jvm` entry covers Maven and Gradle**, detected by `pom.xml`,
`build.gradle`, `build.gradle.kts`, `settings.gradle` or `settings.gradle.kts`,
with one box on the **Language support** pane — a Repo on the JVM picks one
build tool or the other. Neither needed anything but variables: turning
Gradle's daemon off and Maven's locking on are both said in the environment, so
there is no capability of the server's own behind this entry.

| Variable | Where | What moves there |
| --- | --- | --- |
| `MAVEN_OPTS` | `{cache}` | Maven's **local repository**, by `-Dmaven.repo.local`: everything a build downloaded, plugins included, and what `mvn -o` builds out of. And **file locks on it** — see below |
| `GRADLE_USER_HOME` | `{cache}` | **Gradle's whole home**: the dependency caches under `caches/modules-2`, the distributions `gradlew` downloads under `wrapper/dists`, toolchain JDKs under `jdks/`, and the local build cache under `caches/build-cache-1` |
| `GRADLE_OPTS` | — | `-Dorg.gradle.daemon=false`, and no directory at all: **no daemon in a session** — see below |

**Maven's repository is locked with files every session sees.** Maven 3.9's
resolver guards its local repository only inside one JVM unless it is told
otherwise, so two sessions building at once would each take a lock the other
cannot see. `MAVEN_OPTS` also carries
`-Daether.syncContext.named.factory=file-lock` and
`-Daether.syncContext.named.nameMapper=file-gav`, which put one lock file per
artifact under `.locks` in the repository itself — the setting the resolver's
own documentation names for a repository shared between processes. Maven 4
locks with files already; **Maven 3.8 and older have no such locks and ignore
the properties**, so two sessions on a 3.8 share the repository with nothing
guarding it. These are system properties, so a Repo's own
`.mvn/maven.config` naming a `-Dmaven.repo.local` of its own keeps it, and a
`-s` settings file is still read. **A Build Cache whose path has a space in it
gets no shared repository**: Maven 3 splits `MAVEN_OPTS` on whitespace, Maven 4
`eval`s it, and no quoting survives both — so where the path has a space the
whole line is left out, and each session builds with a repository of its own.
`settings.xml`, where a server's password goes, stays in the session's own
`~/.m2`: moving the repository does not move it. **Nor are `mvnw`'s
distributions shared**: the wrapper unpacks a Maven release into the session's
own `/tmp` and moves it into place, and across two mounts that is a copy a
second session could find half-done, so `MAVEN_USER_HOME` is left alone and a
wrapper downloads its Maven once per session.

**Gradle's daemon is off in a session, and that is the price of sharing its
home.** Gradle has no way to move only its caches out of its home, so the home
is shared whole — and a Gradle daemon registers itself under that home and is
reached over the loopback, which every Sandbox shares. With a daemon allowed, a
second session's `gradle` would find the first session's idle daemon and run
its build *there*, inside the first session's Sandbox, where the second one's
Worktree is not bound: it fails with `Could not set process working directory
… could not setcwd()`. That was observed before any of this was built, and it
is the hazard the **Compile Server** exists to remove for sccache. So every
session is given `-Dorg.gradle.daemon=false`, which beats
`org.gradle.daemon=true` in a Repo's own `gradle.properties` or in the shared
home's. Gradle then runs each build in a single-use daemon inside the session's
own Sandbox, gone when the build ends and registered nowhere. **Every Gradle
invocation pays for a JVM starting up**, which is what this costs. [Stage 06 of
the language caches
roadmap](roadmaps/language-caches/06-a-gradle-daemon-of-verksteads-own.md) — a
Gradle daemon Verkstead runs in a Sandbox of its own — is where that may change,
and it may end with the daemon staying off.

**An explicit `gradle --daemon` still gets a daemon.** The command line beats
the environment, so a build started that way registers in the shared home
again. **Only a build that asks for a daemon itself can reach it**: a session
given `-Dorg.gradle.daemon=false` never looks in the registry and starts a
single-use daemon of its own whatever is idle beside it, but another session's
`gradle --daemon` attaches to the one left up and fails with the `setcwd()`
error above. That is accepted
rather than worked around: the failure is loud rather than a build quietly
running in the wrong place, and `gradle --stop` in any session, or the daemon's
own idle timeout, puts it right. Leave
`--daemon` out of a session's commands and out of a Repo's scripts.

**Gradle's build cache is shared only for Repos that switch it on.** Its local
directory is inside the shared home, so every Repo with
`org.gradle.caching=true` shares one build cache for the machine, and a task
one Conversation built comes `FROM-CACHE` in another. Verkstead does not switch
caching on for a Repo that did not ask: that changes how a build behaves, not
only where it writes, and a system property would beat a Repo that wrote
`org.gradle.caching=false` on purpose. A Repo whose `settings.gradle` points
`buildCache.local` somewhere of its own keeps that too.

**A shared Gradle home shares its `gradle.properties` and `init.d/` as well.**
An init script one session writes into `init.d/` runs in every other session's
Gradle builds, and a property written there applies to all of them. This is
the [writable store's bargain](#what-each-language-shares) — one session can
plant a package another installs — extended from packages to build logic, and
it is accepted on the same terms. A Gradle login in `gradle.properties` is
shared with it, so a credential belongs in the Repo's own configuration or in
the environment rather than in the Gradle home.

**Kotlin needs nothing of its own.** It builds through Gradle or Maven and
resolves out of the same stores. The Kotlin compile daemon outlives a build
too, but it finds itself through `~/.kotlin/daemon` under the account's real
home, which no Sandbox binds, so each Sandbox's is its own. **Kotlin/Native's
`~/.konan`**, where it downloads its compilers and platform libraries, is not
in the Build Cache, so it is not shared between Conversations.

## A day's work

**Once per machine: the first start asks.** A Verkstead that cannot do anything
yet is in **Onboarding Mode**, and opens on the wizard at `/setup` rather than
on the workbench. It walks three steps. **What a session needs** names every
dependency it could not find — a sandbox, `git`, one of the four coding agents,
and `gh` as an optional row — with a checkbox where a missing row's tick would
be: the sandbox and `git` start ticked, and Next installs everything you leave
ticked. One password dialog on the machine Verkstead is running on covers the
packages this distribution carries, the vendor installers run after it as you —
Anthropic's for Claude Code and xAI's for Grok Build, landing in `~/.local/bin`
and `~/.grok/bin`, which Verkstead then puts on every session's `PATH` for you —
and a progress bar and a status line say how far it has got. **On a Mac it is
Homebrew's instead**, and there is usually no dialog at all: every ticked row
Homebrew carries is a `brew install` of its own, run as you, Homebrew refusing
to run as root. A Mac with no `brew` yet raises one thing and one only — the
step that makes Homebrew's prefix and hands it to you, which is what Homebrew's
installer would have asked for a password for — and Homebrew installs itself
after it, as you. **Claude Code is the exception there**, and it is Anthropic's
own installer on a Mac as it is everywhere else: Homebrew's `claude-code` cask
was the row for as long as `~/.local/bin` was somewhere a Mac session could not
look, and a Mac session's `PATH` now carries that directory whichever way
Verkstead was started — so what a tick runs is the install that stays current,
and a press with Claude alone ticked installs no Homebrew at all.
**And an Intel Mac is not Homebrew's at all.** Homebrew has dropped those
machines — its installer refuses one outright, and its formulae there get no
bottles — so that Mac has a tab of its own with no `brew` on it, and a press
raises nothing whatever is ticked. What installs is the vendors' own:
Anthropic's for Claude Code, xAI's for Grok Build and OpenCode's own, each
landing under your home and each put on every session's `PATH` for you. **And
`gh` is GitHub's own release**, there being no Homebrew to `brew install gh`
with and no installer script to run: the zip that release carries is unpacked
into `~/.local/bin` as you, at whatever version `releases/latest` points at the
moment you press Next — so it is today's `gh` rather than the one this build
was made alongside. The rest is a sentence on the screen of instructions:
`git` is Apple's command line tools, and `xcode-select --install` opens
Apple's own dialog on that Mac's screen rather than a command Verkstead can
run for you, while Codex is a binary to put in `~/.local/bin`, which every Mac
session looks in.
**And on either Mac, `git` counts only with Apple's command line tools.** Every
Mac has a `/usr/bin/git`, and without the tools behind it that file is a stub
whose whole behaviour is to open Apple's install dialog and exit — so a row
that ticked on finding it ticked on every Mac ever made, and handed each
session of the ones without the tools a `git` that fails. What the row asks is
`xcode-select`, never the stub: running the stub is what opens the dialog, on a
machine nobody is standing at, every ten seconds the wizard re-probes. A Mac
without the tools reads as a Mac with no `git`, with what `xcode-select` said
under the row, and both Mac tabs lead their `git` row with
`xcode-select --install` — Apple silicon keeping `brew install git` beneath it,
that being the `git` a developer's Mac usually runs. A `git` from Homebrew or
from nix is nobody's stub and ticks on being there, the way one does on every
other machine.

Only what could not be installed here — a NixOS, a server with no way to raise
a dialog, an installer that would not run, a Homebrew that could not be
installed, an Intel Mac's `git` — reaches a screen of instructions afterwards,
with this distro's own command and where the binary has to land; that screen
holds Next, counting the rows detected, and it re-probes while you are away, so
an `apt install bubblewrap` finishing in another window ticks the row within ten
seconds. **Agent Profiles** offers the agent accounts already logged in under
the server's home; each one you leave ticked is saved as a Profile with no name
and every model this build knows for that agent — and there is a form under them
for an account elsewhere. **Who the work is committed as** asks for the git
author, prefilled from `git config --global`, with the GitHub token optional and
prefilled from `GH_TOKEN`, `GITHUB_TOKEN` or the host `gh`'s own login, each
field labelled with where its value came from. Above the token field is what one
has to be able to do — `repo` and `workflow` on a classic token, and `gist` as
well to publish a share; Contents, Pull requests, Issues and Workflows to write
and Actions to read on a fine-grained one — because the push that needs them is
made by a session inside the sandbox, where a refusal for a missing scope names
none of them. The settings page says the same above its own token field. The
last Next takes the mode
off and lands you on the compose page. There is no skip and no going back
through it: the verdict is reached once, at startup, so a machine that already
has all three opens the workbench and never sees the wizard. What it does not do
is register the Repos you work in — that stays yours, on the settings page.

A Conversation has a **Process**, which is what kind of work it is: the one
picked decides the shape of the run below and which roles the Conversation is
run under. **Develop** is the full one — the grilling, the Direction and the
pipeline, and the walkthrough below is its. **Tinker** is a Brief that wants no
interview: the press takes it straight into a conversation with one session
about the work itself, and where that conversation built something it ends on a
pull request and the same wrap-up. Both are on the composer's picker; the rest
arrive a Process at a time, and one is offered only once it can run.

A Conversation fixes the **Pairings** the roles it uses need before it starts —
a Profile and one of its models, picked together as one row, on the one
**Agent** control. Develop uses three: a **Grilling Pairing**, an
**Implementation Pairing** and a **Review Pairing**. A Tinker is never
interviewed, so it uses the two a wrap-up needs. The same Profile may fill them
all, and separate ones are how the parts bill to separate accounts. The review one runs the wrap-up's
review and nothing else, reviewing being a fresh set of eyes on what was built —
and its picker offers **No review** beside the accounts, for work you would
rather have wrapped up without one — the one row on any of the pickers that is
not an account. All of them are settled while the Conversation is drafting, and
the work starting is what fixes them.

**Then, per piece of work:**

1. **New conversation**, against a Repo. Pick the **Process** and write the
   **Brief** — the markdown document the work starts from, and its first Event.
   The base commit defaults to the default branch's tip and is yours to
   override.
2. **Start work.** The branch and the **Worktree** are made here, and a
   grilling session opens in the Sandbox. What it wants to know arrives as
   Question Sets on the Timeline and, if you have subscribed, on your phone.
   Answer from wherever you are; the session waits. On a **Tinker** the same
   press skips steps 3 to 5: one session opens on the Brief as the thing to
   follow up on, and the two of you go round in Sets — it does what you asked,
   commits it, and puts the next round to you — for as long as you want, until
   you tick **Nothing else** on one. Then a Conversation that committed
   something carries on into step 6's pull request and the wrap-up under it, and
   one that committed nothing is **Done**.
3. **The Proposal.** The grilling ends by proposing a **Direction** — inline,
   task list or roadmap — on a Set carrying the chooser. Picking one accepts
   the Proposal, and the pick is delivered back to the grilling session rather
   than acted on. Every other way of answering — a different Option, your own
   words, or leaving it open — sends it back, and the session decides for
   itself whether to keep grilling or propose again.
4. **The session produces what you picked.** A task list breaks the work into
   `.tasks/`; a roadmap stages it into `docs/roadmaps/`; inline writes the
   **Handoff** for the fresh session that builds it. That artifact, plus the
   session going quiet, is what ends the grilling and starts the pipeline —
   there is nothing left to press.
5. **It runs itself.** Each **Step** is one fresh session, ended when the file
   it turns on has gone from the Worktree *and* the commit removing it has
   landed *and* the session has gone quiet. Commits appear on the Timeline with
   their diffs. Where Verkstead cannot go on — a session that exited badly, or
   one that landed nothing — it **halts**: a **Notice** on the Timeline says
   what stopped and why, your phone is told, and nothing else is launched until
   you press **Resume**.
6. **The finish runs unattended.** The last Step pushes and opens a **draft
   pull request** per the target repo's `docs/agents/git-workflow.md`, and the
   Conversation moves to Wrapping. The PR is a pinned Event; its commits and
   comments are fetched through the host's `gh`.
7. **The wrap-up settles itself.** A fresh-context session reviews the PR and
   raises what it finds as a Question Set. Failing checks dispatch fix sessions
   — two failed attempts at the same check is where it stops and asks. New PR
   comments dispatch sessions that address them. The Conversation reaches
   **Done** when the checks are green, the review Set is answered, and nothing
   said on the PR is left unaddressed.
8. **Merging is yours.** Done means Verkstead has finished with the work, not
   that it is on `main`. Nothing in the pipeline merges anything.

**On a roadmap**, settling is also what starts the next **Stage**: a
Conversation of its own, primed with the stage brief as its Brief and
Implementing from the first moment. Its branch stands on the predecessor's
wherever the default branch does not yet hold that work — that being where the
work this Stage builds on is — and comes off the default branch once it does.
Where the repository's workflow records how to stack a Stage for review the
session follows it; where it records none the pull request carries the Stage
before it until that one merges. A **Notice** on the Timeline says which Stage
started and where its branch went — or that the roadmap has no Stage left to
run. Nobody presses anything for either.

## What is different in practice

- **Questions belong to a Conversation.** There is no global queue to work
  through: a Set is on the Timeline of the work it came from, and it stays
  there, answered, afterwards. Nothing leaves a Timeline.
- **The checkout is not what gets worked in.** Every Conversation has its own
  Worktree under the Data Directory, so two pieces of work in one Repo no
  longer take turns, and the checkout you have open in an editor is not what a
  session is editing.
- **A run that stops is a thing on a page**, not a terminal you have to find.
  The stop Notice carries the evidence — which Step failed, how it ended, what
  git made of the Worktree, and the tail of what the session last said — read
  at the moment the run stopped and kept. Getting going again is one **Resume**,
  which works out what ought to be running now rather than replaying whatever
  failed; where you want the work to go somewhere else instead, **Steer** is
  what says so — pick the state to carry on in, write the instruction or the
  brief it needs, and the submit both moves the work and sets it going.
- **Review happens once, on the pull request.** This is what "no commit gates"
  buys: nothing pauses per commit, and everything you would have said there is
  said in the wrap-up instead.

## The old tools

`tobico-skills/roadrunner` and the `tobico-scripts` wrappers are left exactly
as they are: still on `PATH`, not deleted, and carrying no deprecation notice
in their own repositories. One person uses them and that person knows they are
retired, so a notice in a repository only they read would be ceremony rather
than warning.

Which also leaves them as the fallback while the switch-over is being made, and
that is the better reason not to touch them: they built this, up to and
including the stage that retires them, and a tool that still runs is worth more
than one removed the day its replacement first worked.
