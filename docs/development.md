# Development

Working *on* Verkstead, rather than with it: running the loop out of a checkout,
and the commands the two halves are built and tested with. There is no other
way in yet — nothing has been released under this name, so a checkout is the
only Verkstead there is.

The vocabulary in bold is the project's, and is defined in
[CONTEXT.md](../CONTEXT.md).

## Quickstart

The whole loop, from a fresh checkout. It takes two terminals: `verkstead ask`
blocks until it is answered, which is the entire point of it.

### 1. Enter the dev shell

```console
$ nix develop
```

Everything below assumes this shell — it carries the Rust toolchain, `sqlite`,
`git`, and the `node` and `pnpm` the viewer is built with.

### 2. Build the viewer and start the server (terminal 1)

```console
$ (cd web && pnpm install && pnpm build)
$ cargo run -p verkstead-cli -- serve --data-dir .
  INFO verkstead_server: verkstead is listening listen=127.0.0.1:8422 peer_listen=0.0.0.0:8423 workbench=http://127.0.0.1:8422/?key=… data_dir=. device=86f1933fecb070cbee865fbb84819d14 fingerprint=3F:0A:… home=/home/you sandbox_binds=0 build_cache=Some("/home/you/.cache/verkstead") skills=./skills
```

**`workbench=` is how you get in.** Every page of the workbench and the viewer's
own `/api/ui/` namespace answer 401 without the **Workbench Key**, and that link
is the address with the key on it: paste it once and the browser holds the
cookie from then on. The key is `workbench.key` in the Data Directory, made at
the first start and read back at every one after it.

**`device=` and `fingerprint=` are what this install *is*.** The id is invented
at the first start and read back at every one after it, and it is what every
record and URL naming a device will name this one by; the fingerprint is its
self-signed certificate's, in the spelling two people compare one in. Both are
in the Data Directory beside the key, as `device.id` and `device.pem`. The
certificate is good for ninety days and the first start with fewer than thirty
of them left makes another ([ADR 0020](adr/0020-cluster-mode.md)) — an expired
one is refused at the handshake, so a certificate issued once and read back for
ever would be the day every link in a cluster went down together. The id is
untouched by that: it is the certificate that is renewed.

While the new certificate is waiting on members to acknowledge it, a line of
its own says so and names both fingerprints — the one still going out and the
one coming in. On a checkout nothing has been linked to, what that line says is
that there was nobody to announce to and the changeover is already over:

```console
  INFO verkstead_server: this device's certificate was near its expiry and has been made again, and there was no member to announce the new fingerprint to fingerprint=9C:4B:…
```

Once something *is* linked — which is further down this step — that start
announces the new fingerprint to every member instead, over the link it already
holds and still presenting the outgoing certificate, that being the only one any
of them holds. The changeover ends at the last acknowledgement; a member that
was switched off is told by the next call that gets through to it, and one that
never answers is a member the human unlinks.

**`peer_listen=` is where another Verkstead reaches this one.** A second
listener, TLS on every interface at port 8423, presenting the certificate
above and asking a caller for one without insisting on it — `--peer-listen` or
`VERKSTEAD_PEER_LISTEN` moves it, and a second Verkstead on this machine needs
its own the way it needs its own `--listen`. The one route on it anybody at all
may read is the identity endpoint —

```console
$ curl -k https://127.0.0.1:8423/api/peer/v1/identity
{"device":"86f1933fecb070cbee865fbb84819d14","fingerprint":"3F:0A:…","name":"workbench",
 "os":"Linux","addresses":["workbench.tailnet-name.ts.net","100.64.0.1","192.168.1.24"]}
```

`-k` because the certificate is self-signed and made out to the device id
rather than to an address: in a cluster what proves the far end is that
fingerprint compared against the one the other machine printed, and there is no
certificate authority anywhere in it to check a chain against.

The three after the fingerprint are read off the machine as that request is
answered rather than configured anywhere: `name` is the hostname, `os` is the
platform's own word — a WSL reads `Linux (WSL)`, a Windows machine and the WSL
on it sharing a hostname — and `addresses` is everywhere a peer could reach this
device, the tailnet name and address first where Tailscale is up and the LAN
behind them. Ask it again from another machine on the same tailnet and it says
the same thing; ask it off a laptop that has moved and the addresses have
moved with it.

**And the same device says so on the LAN, so that nobody has to type any of
that.** A start advertises `_verkstead._tcp.local` over mDNS — in this process,
with no avahi or Bonjour to install — carrying the device id, the hostname, the
OS word and the port the peer listener really landed on. Anything that browses
mDNS reads it back:

```console
$ avahi-browse -rt _verkstead._tcp
= enp10s0 IPv4 86f1933fecb070cbee865fbb84819d14  _verkstead._tcp  local
  hostname = [86f1933fecb070cbee865fbb84819d14.local]
  address = [192.168.1.24]
  port = [8423]
  txt = ["port=8423" "os=Linux" "name=workbench" "id=86f1933fecb070cbee865fbb84819d14"]
```

The instance is named by the device id rather than by the hostname, because two
Verksteads on one machine are two devices and a hostname cannot tell them apart —
start the second one below and this lists two, each naming its own peer port.
`--no-advertising` or `VERKSTEAD_NO_ADVERTISING=1` turns it off, and on NixOS
`services.verkstead.advertising = false;` does: what goes out is a hostname, an
operating system and a device id, on a LAN that may not be yours. A server
*asked* to stop — a `SIGTERM`, or a `^C` — withdraws the advertisement on its way
out, which is the one ordered stop this server has; a killed one leaves the row
on the other machine to run out on its own TTL, the way a shut lid does.

**And the other half of it is what the pane draws under those rows.** The same
service browsed rather than advertised, which is the **Discovered** list: every
device this one has found and is not already in a cluster with, each with every
address it was found at and an **Add** on the row.

```console
$ curl http://127.0.0.1:8422/api/ui/devices/discovered
[{"device":"0011223344556677889900aabbccddee","name":"kitchen-mini","os":"macOS",
  "addresses":["192.168.1.31:8423","100.64.0.9:8423"],"found":["Lan","Tailscale"]}]
```

**The browse runs while that list is being read and not otherwise.** It starts on
the first read of it and stops once nothing has read it for five minutes — a phone
that closes a tab says nothing, so the reading being read is the whole of what
governs it. Which means the first read is empty or short however many machines are
out there: a browse is cold when it starts, and the rows arrive over the seconds
after it, each as a `discovered` nudge that an open pane redraws on.

The rows are the Nudge's and nothing polls for them. The one interval in the
viewer is on this read alone, once a minute while the pane is open, and what it is
for is the spell above rather than the rows: a browse that has heard nothing new
announces nothing, so without it the server would stop browsing five minutes into
a pane somebody was still watching, and a second Verkstead started after that
would never be heard. A test reads the list again itself, which renews the spell
the same way.

**And `found` is a list because there are two ways of being found.** A tailnet
carries no multicast, so there is nothing to hear on one: the tailnet half asks
instead, reading the online peers out of `tailscale status --json` and putting the
identity endpoint's question to each of them on port 8423 as the list is read. So
the tailnet rows are in the first answer where the LAN rows arrive after it, and a
machine on this network *and* this tailnet is one row that says `Lan` and
`Tailscale` both, its LAN address first. Bounded, because how many nodes a tailnet
has is nobody here's decision: sixty-four peers at most, sixteen at a time, three
seconds apiece. `RUST_LOG=verkstead_server::discovery=debug` says which peers were
asked and what each of them answered — a phone or a server with nothing on that
port is a debug line and no row.

Three kinds of device are left out of the merged list — a member, this device, and
one a join is already pending for — so what the list holds is only what there is
anything to press. Start the second Verkstead below with a data directory of its
own and this lists it; link the two and it is a member above instead.

**Three routes stand outside the member gate and they are the whole of the
un-gated surface**: that identity endpoint, the join post, and the cancel and
the dial-back a join is settled through. Every other path on that port answers
`403` and says so — *you are not a member of this verkstead's cluster* —
whatever you present and whether or not a route answers it, so a stranger is
told it is a membership they are missing rather than a path that is not there.
Behind the gate is one membership said three ways: a device put on this one's
list, a device taken off it, and the certificate one of them stands under
changed.

**Seeing a link made takes a second Verkstead**, which on one machine means a
second of everything: its own Data Directory, its own workbench port and its own
peer port. In a terminal of its own —

```console
$ mkdir -p /tmp/other
$ cargo run -p verkstead-cli -- serve --data-dir /tmp/other \
    --listen 127.0.0.1:8522 --peer-listen 0.0.0.0:8523
```

Two installs, two device ids, two certificates. Open the **Remote access** pane
on the first one's workbench: its **Devices** section holds one row, marked *this
device*, and under **Discovered** the second install appears within a second or
two of the pane being opened — heard over the multicast, drawn with the port its
listener bound, and with an **Add** on the row. It reads *LAN* and not *Tailscale*
however much Tailscale is on this machine: the tailnet half asks this machine's
*peers*, and the second install is on this machine. Two machines on one tailnet are
the case that reads *Tailscale*, and *LAN and Tailscale* where they share a network
too.

**Press that Add and nothing is typed anywhere.** The press names the device
rather than one of its addresses, a discovery having found a list of them: the
server dials every address on the row in the order it found them — the LAN's
first, that being the shorter road — and posts the join at the first that answers.
The box under the list is what is left for the devices neither half reaches, and
it takes the one address it always did: `127.0.0.1:8523` for the second install,
the port being needed only because both are on this machine.

What happens then is the whole of the stage, and the two presses are one act from
here on. The first device dials that address, takes whatever certificate it
presents for the one call, and posts what it is; the second writes the question
down, holds it ten minutes, and raises a
modal in every workbench it has open with a push to any phone subscribed to it.
The first draws a pending row, *Waiting for confirmation on …*, with **its own**
fingerprint under it — the same string the modal over there is drawing, for two
people at two screens to compare by eye — and a **Cancel**.

Press **Allow** on the second one's modal and nothing else is pressed anywhere.
The second dials the first back, checks the certificate it meets is the one the
request pinned, and hands over itself and every member it holds; the first
checks that certificate against the one it met when it asked. Both lists now
read the same, and a third Verkstead joining through either of them lands on all
three.

The pending row and the discovered row are never both drawn: a device a join is
pending for is one the Discovered list leaves out, so the press moves a row from
under the list to above it. And a row that went stale between being drawn and
being pressed — the machine switched off in between, so nothing answers at any of
the addresses it was found at — is refused naming the device and dropped from the
list, rather than sitting there refusing again. A far end that *answered* and said
no keeps its row, being exactly where the row said it was: only a press that
reached nobody says the row was wrong.

Through the API rather than the pane, which is what a test does — the discovered
press first, then the typed one:

```console
$ curl -X POST http://127.0.0.1:8422/api/ui/devices/discovered/0011…ee/add
$ curl -X POST -H 'Content-Type: application/json' \
    -d '{"address":"127.0.0.1:8523"}' http://127.0.0.1:8422/api/ui/devices/joins
$ curl http://127.0.0.1:8522/api/ui/devices/asking
[{"request":"5b1f…","identity":{"device":"86f1933f…","fingerprint":"3F:0A:…",
  "name":"workbench","os":"Linux","addresses":["192.168.1.24"]}}]
$ curl -X POST http://127.0.0.1:8522/api/ui/devices/asking/5b1f…/allow
$ curl http://127.0.0.1:8422/api/ui/devices
```

**Unlink** is the row's other press, and it takes that device out of the cluster
for everybody rather than cutting this device's own half of a link: every member
drops it and the device itself is told to forget the rest. It is asked once,
over the page, as Remove on a Repo is — and it works on a member that is not
answering, which is most of what it is for. A member the last dial found nothing
at stays on the list, dimmed, reading *unreachable*.

A device asking to link is the one thing a stranger writes into this machine, so
it is bounded at both ends: a post saying more about itself than is kept is
refused, and so is one that would take this device past sixteen questions held
at once.

**Once two devices are linked, either one's whole workbench is reachable through
the other.** A member serves `/api/ui/` over its peer listener behind the member
gate, and the device the browser opened relays for the rest: everything under
`/api/ui/members/{device}/…` is put to that device verbatim — method, path,
query, body and the headers that matter — and its answer comes back untouched,
status and body and all. The prefix takes the place of `/api/ui`, so
`/api/ui/members/0011…ee/conversations/4` is that device's own
`/api/ui/conversations/4` and nothing else. The browser stays same-origin
throughout and a device's workbench key never leaves it: what admits the hop at
the far end is this device's certificate, and the cookie is not passed on.

```console
$ curl http://127.0.0.1:8422/api/ui/members/0011…ee/conversations
$ curl -X POST -H 'Content-Type: application/octet-stream' --data-binary @notes.md \
    http://127.0.0.1:8422/api/ui/members/0011…ee/conversations/4/attachments/notes.md
```

The body is streamed rather than held, in both directions, so an attachment is
an ordinary post here and the limit that refuses an oversized one is the far
end's own `413` rather than a judgement made after buffering the file. Three
Device Ids are refused by name instead of dialled: one that is no member's, this
device's own — local URLs keep their shape, so nothing should ask — and a member
that answered at none of the addresses it advertised, which is the row the pane
is already drawing dimmed.

**And the news comes back the same way.** This device holds one Nudge stream to
each of its members — that member's own `/api/ui/nudges`, read over the peer
listener — and announces everything down it on the stream its own pages are
listening to, under the Device Id it came from. So a page drawn on a member's
Conversation stays fresh without a poll and without a reload, and a phone on the
tailnet hears about that machine at all. The streams are the server's rather than
the browser's: one per member serves every page this device has open.

```console
$ curl -N http://127.0.0.1:8422/api/ui/nudges
event: nudge
data: {"kind":"set","conversation":4}

event: nudge
data: {"kind":"set","conversation":7,"device":"0011…ee"}

event: nudge
data: {"kind":"everything","device":"0011…ee"}
```

A Nudge with no device is this device's own and is the frame it always was; one
with a device is that member's news, and the viewer's own table keys the reads it
makes by it. The `everything` kind is the stream itself rather than anything in
the world: a member's stream that has just been taken up knows nothing about what
it missed, so it says *read back whatever of this device is on screen*. Nothing at
all is announced for a member that is not answering: the page keeps what it last
read and goes stale, exactly as it does when its own stream is down. A stream that
ends is taken up again five seconds later, and while nothing answers at all that
wait doubles to a minute — a laptop that is shut for a fortnight is worth a dial a
minute rather than one every five seconds. What goes over the
peer listener is this device's own news alone: in a cluster everybody holds a
stream to everybody, so a device passing on what a third one told it would be
saying that news was its own.

**That is the whole of it — there is no boundary flag to say.** A repo is
registered from anywhere the server can read, an **Agent Profile** names an
account anywhere the server can read, and every path field browses the same
way, opening on the server's own `HOME` when nothing has been typed into it.
What keeps a session to its own Conversation is the **Sandbox**, composed from
the Repo and the Profile that Conversation names.

The one path list left is the **Sandbox Configuration** binds, and they are
said in two places — `--sandbox-bind DIR`, and the same grammar on the settings
page's **Sandbox binds** section. What a session gets is the union; see the
`"paths"` payload further down this section. The flag is the shape a service unit wants,
where startup is the moment to hear about a typo and a bind that is not there
refuses to start; the settings are the shape a bare binary wants, where a save
has to land whatever it was told and an entry that will not resolve is reported
rather than fatal.

Everything Verkstead makes goes in one place, the **Data Directory**: the
database at `verkstead.db`, the worktrees, the installed skills, the handoff
directories, the files this device is, and the settings files. `--data-dir`
says where, or `VERKSTEAD_DATA_DIR`. Said nothing, it is the platform's own
place for it — `~/.local/share/verkstead` on Linux, `~/Library/Application
Support/Verkstead` on macOS — which is what an installed Verkstead wants and
not what a dev run out of a checkout does: `--data-dir .` is why every command
here says it, and it keeps the database, the worktrees and the settings beside
the checkout where they can be deleted with it.

The desktop app is a verb of that same binary, and the same server: `cargo run
-p verkstead-cli -- desktop --data-dir .` serves what the command above serves
and opens the viewer in your browser as it comes up. `--no-open` leaves the
browser alone, and every other flag is the server's own, because the app *is*
the server ([ADR 0012](adr/0012-desktop-tray-binary.md), as amended) — started
with nothing said it is the platform's Data Directory again, which is what a
machine that installed it wants and not what a checkout does. The tray half is
`crates/desktop`, a library the CLI carries behind its default-on `desktop`
feature: a build that says nothing gets both halves, which is what makes every
image that can serve one that can also `ask`, and `--no-default-features` is
the headless build the musl CLI and the nix package take. It is the one crate
here that links a system toolkit — GTK on Linux, which is why the workspace
builds in the dev shell and nowhere else here; AppKit on a Mac and Win32 on
Windows, which are those platforms' own and want nothing installed. An address
something is already listening on — the command above, say — is a dialog and a
nonzero exit rather than a second Verkstead beside the first.

What it puts on the screen is an icon in the system tray, and the menu on it is
**Open** — the viewer again, in your browser — **View Logs**, which opens the
file the server's log goes to instead of a stdout nobody launched from an icon
will read, **Launch on Startup**, and **Exit**, which stops Verkstead where it
stands the way stopping the systemd unit does. Run it where there is no screen
to put an icon on, over SSH or under a test, and it is the server and the open
and no more: a warning in the log, and everything else exactly as it was.

**Launch on Startup** is a checkbox over the platform's own registration — your
desktop's autostart entry at `~/.config/autostart/net.tobico.Verkstead.desktop`
here, a launch agent at `~/Library/LaunchAgents/net.tobico.Verkstead.plist` on
macOS, a `net.tobico.Verkstead` value under
`HKCU\Software\Microsoft\Windows\CurrentVersion\Run` on Windows — and that
registration is the whole of the state: checking the box writes it, unchecking
removes it, turning it off in your desktop's own settings unchecks it, and no
setting of Verkstead's own keeps a second copy of the answer. Every
launch rewrites it while it is there, with the path of the executable that is
running and the `desktop` verb behind it — one image has more than one way in
now — so a binary you moved heals its own entry the next time you start it by
hand. What it writes starts the app with `--no-open`: a login is not a
moment to be handed a browser window. The one thing the box cannot see is the
platform's own second opinion about it — macOS's Login Items list, which
`launchd` keeps in a database rather than in the file, and Windows' Startup tab
in Task Manager, which Explorer records under `StartupApproved`: switch
Verkstead off in either and the box goes on showing what the registration says.

One directory is made outside it: the **Build Cache**, at
`$XDG_CACHE_HOME/verkstead` — `~/.cache/verkstead` on most machines — unless
`--build-cache-dir` says otherwise. Every session gets it writable, with
`CARGO_HOME` inside it, so a crate is downloaded once for the machine rather
than once per Conversation; with `sccache` on the `PATH` the server was started
from, every session is told to compile through it as `RUSTC_WRAPPER` and the
compiling is cached too, on all three platforms. The dev shell carries one, so a
checkout run gets the whole thing. It is on with nothing configured, and the
settings page is where it is switched off or given a size.

The sccache **server** is Verkstead's own, not the sessions'. It comes up as a
child of the running server the first time a session starts on a repo with a
root `Cargo.toml`, in a sandbox holding `<data-dir>/worktrees` and the build
cache and nothing else — so `ps` shows one more sandboxed child beside each
session's, and it goes when the server does. That sandbox is described and
rendered by the code a session's is, so it is `bwrap` on Linux, `sandbox-exec`
on a Mac and the session account on Windows without any half saying which.
Every session's `sccache` is only the client half reaching it. Sessions starting
their own is what this replaces: they all bind one port, and the loser's
compiles then run in the winner's sandbox where its worktree is not reachable.

**Windows had neither half for a while**, and the reason has gone. A session
there ran inside an AppContainer, which is refused every connection to the local
machine — the probe behind [ADR 0014](adr/0014-windows-sessions.md) timed out on
`127.0.0.1` and on the machine's own LAN address alike, and the sccache client it
ran panicked reading its own configuration before it got that far. A session
runs as a local account of Verkstead's own now: an ordinary local account
reaches the loopback like anything else, and the profile it is handed is a real
one with both halves made, which is what that client was looking for. So the
switch is on there like everywhere else, and the compile server on that platform
runs as the same account a session does, behind entries of its own — a compile
server started as the human would be every dependency's proc macro running with
the database and the settings files in reach.

**And a Windows session finds the machine's Rust through rustup.** There is no
`/nix` on that platform for a toolchain to be on, so what a session runs is the
human's own: `~/.cargo/bin` is on its `PATH` already, being under the home, and
the rustup home those shims resolve their toolchain out of is granted read-only
beside it with `RUSTUP_HOME` naming it — a sandbox has a profile of its own, so
rustup left to resolve against that one answers *could not choose a version of
rustc to run*. `RUSTUP_HOME` where the machine sets one, `~/.rustup` where it
does not, and nothing at all where there is no rustup on the `PATH`, which is
every nix machine.

A session's GitHub auth and the author of its commits are two of those settings
rather than anything found in a home directory. Put a token in `secrets.yaml`
beside the database, and who you are in `config.yaml`:

```yaml
# secrets.yaml
github_token: ghp_...
```

```yaml
# config.yaml
git_author:
  name: Tobias Cohen
  email: tobi@tobico.net
rust_build_cache:
  enabled: true
  size: 30G
cleanup:
  trim:
    enabled: true
    days: 3
  delete:
    enabled: false
    days: 30
conflict_resolution: merge
share_on_done: false
sandbox_binds:
  - /var/cache/verkstead-node
  - /var/cache/verkstead-cargo
```

`sandbox_binds` at the foot is the other place the Sandbox Configuration binds
are said. A bind is one absolute path, every session gets every one of them, and
what the server goes by is the union of this file and the installation's own
flags.

Every session started after that gets the token as `GH_TOKEN`, which `gh`
honours without being told to — as does the server's own `gh`, the one that
reads a pull request's checks, commits and comments onto a Timeline — and gets
git configured through the
environment — the author, `gh auth git-credential` as the credential helper for
GitHub, and SSH GitHub remotes rewritten to HTTPS so a `git push` inside
authenticates with the token. There is no file to write and nothing to log in
to inside a sandbox. With no token — no file, an empty one, one that will not
parse — sessions start anyway and `gh` inside says it is not logged in, and the
server's own `gh` falls back to whatever login the host has; with no author, git
inside asks to be told who you are.

Either file can be written through the viewer's API instead of by hand, which is
what the settings page saves through:

```console
$ curl http://127.0.0.1:8422/api/ui/settings
{"git_author":{"name":"","email":""},"github_token":null,
 "rust_build_cache":{"enabled":true,"size":"30G","size_configured":false,
   "compiles":"Cached"},
 "cleanup":{"trim":{"enabled":true,"days":3,"days_configured":false},
   "delete":{"enabled":false,"days":30,"days_configured":false}},
 "conflict_resolution":"Merge",
 "share_on_done":false,"paths":{"binds":[]}}
$ curl -X POST -H 'Content-Type: application/json' \
    -d '{"git_author":{"name":"Tobias Cohen","email":"tobi@tobico.net"},
         "github_token":{"Set":{"token":"ghp_..."}},
         "rust_build_cache":{"enabled":true,"size":""},
         "cleanup":{"trim":{"enabled":true,"days":""},
           "delete":{"enabled":false,"days":""}},
         "conflict_resolution":"Merge",
         "share_on_done":false,
         "sandbox_binds":["/var/cache/verkstead-node"]}' \
    http://127.0.0.1:8422/api/ui/settings
{"settings":{"git_author":{"name":"Tobias Cohen","email":"tobi@tobico.net"},
  "github_token":{"last_four":"cdef","at":"2026-08-23T08:23:15.041950412Z"},
  "rust_build_cache":{"enabled":true,"size":"30G","size_configured":false,
    "compiles":"Cached"},
  "cleanup":{"trim":{"enabled":true,"days":3,"days_configured":false},
    "delete":{"enabled":false,"days":30,"days_configured":false}},
  "conflict_resolution":"Merge",
  "share_on_done":false,
  "paths":{"binds":[{"path":"/var/cache/verkstead-node","repo":null,
    "source":"Settings","resolution":{"Unresolved":{"why":
      "the server cannot see it: there is nothing at that path"}}}]}},
 "verified":{"Account":{"login":"tobico","missing":["gist"]}}}
```

The token goes one way. What comes back about it is its last four characters and
when `secrets.yaml` was written, never the token itself. Saving one asks GitHub
who it authenticates as and answers with the account or with what went wrong —
and writes it down either way, because a token is pasted once out of a page that
will not show it again, and a network that was briefly down is no reason to send
somebody back for another. `"missing"` beside the account is the scopes
Verkstead needs that GitHub says the token has not been given — `gist`, which
publishing a share writes with, and empty on a token that carries it or on a
fine-grained one GitHub named no scopes for at all. `"github_token"` is
`"Keep"` to leave the configured one alone, which is what a save of the author
fields sends, and `"Clear"` to take it away. `"rust_build_cache"` is a pair of
values rather than an action: an empty `"size"` is no size configured, which
puts the default back, and `"compiles"` is read-only — `"Cached"` where the
server found an `sccache` and `"NoSccache"` where it did not. Its own
environment rather than anybody's setting.

`"sandbox_binds"` is the list `config.yaml` holds, sent as values in the grammar
the flags use — so a Verkstead started with no flags at all gets its first bind
through this endpoint. What is sent is what the file holds afterwards, and the
`"paths"` that comes back is both sources at once: every entry says whether the
installation's flags or the settings said it, and whether the server can see
what it names right now. Only the settings' own can be sent, and nothing about
them is checked as it is written — the save lands whatever it was told, and an
entry the server cannot see is a `"resolution"` saying so rather than a refusal.

A published share is read through the **share viewer**, a small static page that
draws it in a browser — a gist link on its own shows source. It is not
configurable and there is nothing to configure: Verkstead keeps the one copy on
its own GitHub Pages, at
<https://tobico.github.io/verkstead/share-viewer.html>, and every link it hands
out — the toast, the Share pane and the comment on a pull request — is composed
through that, as `<share-viewer-url>#<gist-id>`. The page is published by
`.github/workflows/pages.yml` whenever `crates/server/share-viewer.html` lands
on `main`; its address is `HOSTED` in `crates/server/src/sharing.rs`, and
`web/tests/viewing.test.ts` is what holds the two spellings together.

Nothing about that is secret: the page is public, the id after the `#` rides in
the fragment and is never sent to the host that serves it, and the share itself
is fetched from GitHub by the reader's own browser.

`"share_on_done"` is whether a conversation's record is published and linked on
its pull request when the work settles to Done, which is otherwise something the
human presses for. It is the one setting here that is **off** with nothing
configured — an absent key, an absent file and one nothing can parse all mean
off — because what it turns on publishes a gist under the human's own account
and comments on a pull request other people are reading. It is set from the Git
pane, beside the token it is published with.

What it turns on is the wrap-up's own settle rather than the state: a steer into
Done shares nothing. It fires once per conversation, gated on the row in
`share_comments` that says a share-to-PR comment has been left on it — written
wherever a comment lands, the Share pane's press and the settle alike, and never
deleted. So a conversation shared by hand stays quiet, and a share that failed
wrote no row and is tried again at the next settle. A failure is a Notice on the
timeline naming it; a share that worked writes nothing there.

`"cleanup"` is what becomes of a conversation the human has archived: a **trim**
that takes its bulk — the full agent output, the transcripts and the session
records, which is everything a share never carried — and a **delete** that takes
the whole conversation. Each is a switch and a number of days, each clock counts
from the archiving, and unarchiving stops both. The two default the opposite
ways about, and an absent key, an absent file and one nothing can parse all say
it: the trim is **on** at three days, because what it takes is what nobody opens
twice, and the delete is **off** at thirty, because it is the one thing here
that forgets. A duration that is not a whole number of days — an empty field
included — is no duration configured, which is how the default is asked for
back; `"days_configured"` beside the number is what says which of the two is
being shown, and the settings page draws the default as a placeholder. Nothing
here is refused, a delete sooner than the trim included: the clocks are
independent, so the conversation is simply deleted before it was ever trimmed.
The sweep reads the file on every pass, an hour apart, so a switch flipped from
a phone is in force on the next one without a restart. It reports what it did in
the log and asks nothing of anybody — no timeline card, no notice — while
announcing on the nudge stream that the record moved, so an open page stops
drawing what has been taken. A pass that took something ends with a `VACUUM`,
because SQLite frees the pages a delete emptied inside the file and leaves the
file the size it was; a pass that found nothing does not. And it touches nothing
outside Verkstead's own record: the git branch stays, and a published share
stays published.

`"conflict_resolution"` is what a session sent at a pull request that will not
merge is told to do about it: `"Merge"`, which merges the base branch into the
work branch and pushes, or `"Rebase"`, which rebases the branch onto its base and
force-pushes it with `--force-with-lease`. An absent key, an absent file and one
nothing can parse all mean a merge — a rebase rewrites what reviewers have
already read and breaks anything stacked on the branch, and nobody should meet
that for never having found the settings page. In `config.yaml` the word is
lowercase, as `merge` or `rebase`.

That word is the whole of the answer, in every repo. One repo could once say
otherwise, from its own pane on the settings page; an override no settings page
draws would be a repo rebasing with nowhere to read why, so it is gone and the
file is the only place this is asked. It is picked on the Git pane, under the
author — a select of two words, saving on the pick.

The link is composed as a page is drawn rather than written down at the publish.
What the record holds is the gist's own URL, so a share published before there
was a viewer to compose through links through one now, and the viewer moving
retargets every link there is without republishing anything. The gist's own URL
travels beside it to the workbench, because the Share pane draws both: the
viewer's link is what a reader is sent, and the gist is where the file is and
the only place a share can be deleted — Verkstead deletes none of them.

Everything about sharing is on one pane of the workbench, opened by the share
icon on the timeline pane's header: where the last share went, and the download,
the publish and the share to the pull requests under it. It was four rows of the
conversation's actions menu, and none of them did anything to the conversation,
which is what that menu is for.

That link is what **Share to pull request** leaves behind. One press on a
conversation whose work is on a pull request publishes a share and comments on
every pull request the conversation holds — its own repository's and each
companion's — carrying the link and an itemized summary of what is in the file.
Comments only: nothing edits a description, and sharing again leaves another
comment rather than rewriting the one before it. A pull request the comment
could not land on is named beside the ones that worked.

One binary serves both halves: the agent API under `/api/v1/`, and the web UI
on <http://127.0.0.1:8422/>. It creates `verkstead.db` in the Data Directory on
first run, which `--data-dir .` above makes the checkout. Leave it running;
check it in a third terminal if you like:

```console
$ curl http://127.0.0.1:8422/api/v1/health
ok
```

The viewer is built into the binary, so `pnpm build` is what puts a UI at that
address. Skip it if only the API matters — the server starts either way, and
says on every page that the viewer was not built. While working on the viewer
itself, `pnpm dev` is the better half of this: see [The dev loop](#the-dev-loop).

### 3. Start a conversation (in the browser)

Every Question Set is asked *from* a Conversation and lands on its Timeline, so
there has to be one before an agent can ask anything. Open
<http://127.0.0.1:8422/> and press **New conversation**. That opens the composer:
drop the **Repo** dropdown along the bottom of the box and press **Open repo**,
which takes any git repository root the server can read — the path field's
dropdown opens on your home to browse for one — and puts the draft on it. Then
press **Save as draft**.
The URL then names what it made — `/conversations/1` — and that number is the
one below.

The same two steps over the API, which is what a script does:

```console
$ curl -X POST -H 'Content-Type: application/json' \
    -d '{"path":"'"$HOME"'/src/verkstead"}' \
    http://127.0.0.1:8422/api/ui/repos
"Added"
$ curl -X POST -H 'Content-Type: application/json' \
    -d '{"repo_id":1}' http://127.0.0.1:8422/api/ui/conversations
{"Started":{"id":1}}
```

### 4. Ask (terminal 2)

```console
$ export VERKSTEAD_SERVER=http://127.0.0.1:8422/conversations/1
$ cargo run -p verkstead-cli -- ask examples/questions.yaml
```

`VERKSTEAD_SERVER` is the whole of what says which Conversation is asking. A
real session never sets it by hand: the orchestrator injects it into the
sandbox, scoped to the Conversation the session is running for, so the bundled
CLI attributes every Set explicitly without knowing it is doing so. Nothing is
inferred from the project or the branch — two Conversations against one repo
would be indistinguishable by either.

A wait that goes to plan is silent, and the little the CLI does have to say —
reconnecting, or refusing a Set — is on **stderr**, written as a YAML comment.
Stdout carries the Response and nothing else, so an agent can parse it as it
stands, even out of the one file its harness merged both streams into.

The command does not return. It has submitted
[`examples/questions.yaml`](../examples/questions.yaml), along with the project
and branch it derived from this working directory, and is now holding a
long-poll on Question Set 1. There is no timeout: only an answer or a kill ends
the wait ([ADR-0001](adr/0001-blocking-cli-for-agent-integration.md)).

The **Diff** on the Set is not the CLI's doing: the server reads it off the
Worktrees as the Set arrives — the Conversation's own, and each read-write
companion repo beside it, one labeled block each. A Conversation started this
way has never been grilled and so has no Worktree at all, which is why the Set
below carries no Diff — one asked from inside a real session carries whatever
that session has left uncommitted, in whichever repository it left it.

A Set can also arrive on stdin, which is how an agent usually sends one:

```console
$ cat examples/questions.yaml | cargo run -p verkstead-cli -- ask
```

### 5. Answer (in the browser)

This is the human's part. Open <http://127.0.0.1:8422/> and pick the
Conversation the Set was asked from: it is on that Conversation's Timeline,
summarised as the table of number, question and answer, and badged `waiting on
you` — the CLI is still holding its long-poll. Press the row and the details
pane is the whole ask.

(A Set is also a page of its own at `/sets/<id>`, which is what a push
notification opens on a phone. It draws the same sheet and answers through the
same endpoint; what it has that the pane does not is a way back to the
Conversation it belongs to.)

The sheet is the whole ask: the Preface, the Diff of the Worktrees the agent
asked from, and each Question with its Options. Pick one, or write your own
words, or both — an Option with a ★ is the agent's Recommendation, and
**Accept all ★ Recommendations** fills in every question you have not answered
yet. Leave a question alone to send it back open; **Submit** asks you to
confirm that before it goes.

Under the last Question is the comment box, for anything about the Set as a
whole rather than about one Question — and directly above it, where the agent
wrote one, the **Postscript**: what it wanted to raise without making a
Question of it, which the example Set closes with. Nothing there has to be
answered, and an empty box says there was nothing to add.

The same Response can go in over the API instead, which is what an integration
test or a script does — see
[`examples/response.yaml`](../examples/response.yaml), which answers every
Question in the example Set, leaves one explicitly open, and adds a set-level
comment.

### 6. Delivery

Back in terminal 2, the still-waiting CLI has printed the Response and exited —
this is what it prints for the answers in `examples/response.yaml`:

```console
answers:
- label: Q1
  selected: 1
  free_text: Start in-process. Revisit it the day we run more than two instances.
- label: Q2
  selected: 2
- label: Q2a
  unanswered: true
- label: Q3
  free_text: acct_8f21c3, the nightly export job in `ops/export`.
comment: |
  Ship it behind a flag and turn it on for the one noisy client first.

  That export job hammers the endpoint on purpose, so give it an allowlist
  entry rather than a bigger bucket for everyone.

  On Q2a I genuinely don't know — pick whatever our SDK's retry logic
  already understands, and say in the PR which one that was.

$ echo $?
0
```

That is the loop. Run step 4 again and Question Set 2 appears on the same
Conversation's Timeline, to be answered the same way. To answer it from your
phone instead, open the settings page's **Remote access** section: it says what
this machine's Tailscale is doing, its checkbox puts the tailnet name in front
of the port this server is listening on — the HTTPS push notifications need to
work at all — and the login link is drawn there as a QR code to point the
phone's camera at, the address and the **Workbench Key** together. Nothing to
run in a terminal, and nothing to paste: Tailscale itself is the machine's, and
a serve set up by hand reads on that page exactly as one set up from it would.

Where the press is refused for want of the operator grant — Tailscale allows a
serve from nobody but root and the tailnet's operator — the pane shows the
`sudo tailscale set --operator=…` that lifts it, and the next press is the
re-try. A `cargo run -p verkstead-cli -- desktop` puts that through `pkexec`
instead, an app having somebody at the machine to ask where a `serve` started
in a terminal has not.

## The dev loop

```console
$ cargo test              # unit, schema and end-to-end tests
$ cargo clippy --all-targets
$ cargo fmt
$ nix fmt                 # the Nix files
$ nix flake check         # the viewer's suite, and the NixOS module in a VM

$ blender -b tools/hammer/verkstead-hammer.blend \
    --python tools/hammer/render.py   # the artwork, from the blend file it is modelled in
$ tools/generate-icons.sh     # the favicon and PWA icons, after re-rendering the artwork
$ tools/generate-packaging.sh # the desktop entry, the launcher icons, the icns and the ico
$ tools/build-appimage.sh     # Verkstead-x86_64.AppImage, once the viewer is built
$ tools/build-macos-dmg.sh    # Verkstead-universal.dmg, on a Mac
$ tools/build-windows-msi.sh  # Verkstead-x86_64.msi, on Windows
```

The last three are the three desktop artifacts a release ships, one per desktop
platform. Each takes everything from the working tree and leaves one file under
`target/`, and each wants `web/dist` already built, because the viewer is
compiled into the binary they wrap. Each also runs only where its artifact does:
the dmg wants a Mac for `lipo` and `hdiutil`, and the msi wants Windows for the
WiX toolset and the MSVC build under it, so the dev shell has the first of the
three and nothing of the other two.

The AppImage is the unified binary, the packaging assets and every library the
tray is drawn over, in one file, and it is the same command CI runs. It builds
what `cargo build --release -p verkstead-cli` builds, feature and all, and its
`AppRun` supplies the `desktop` verb — a desktop launcher names a file and has
nowhere to say one — so it wants the dev shell for the same reason that build
does.

The dmg is `Verkstead.app` — the same binary built for both Apple targets and
`lipo`-ed into one, the icns from `packaging/`, and an `Info.plist` whose
`CFBundleIdentifier` is `net.tobico.Verkstead` and whose `LSUIElement` is what
makes it a menu-bar app with no Dock tile. Its `CFBundleIconFile` names that
icns with the extension on it — macOS appends `.icns` only to a value that has
none, and a dotted identifier reads as having one already, so a value without
it is a name no file answers to and Finder draws the generic app icon. The
release's dmg leg reads the key back off the mounted bundle and has `iconutil`
open what it names, which is the only way that failure is visible from outside
a Finder window. It runs on a Mac only: `lipo`, `codesign` and `hdiutil` are
the operating system's own tools, and there is no cross build of it from here.
The bundle is ad-hoc signed rather than signed with a Developer ID, because
Apple silicon will not execute a binary with no signature at all — that is not
the signing that gets an app past Gatekeeper, and there is none of that.

The msi is the two files a Windows install is — the unified `verkstead` and the
windows-subsystem shim that opens it from an icon — wrapped in an installer,
because two files beside each other are not a portable download. What goes where
is [`tools/verkstead.wxs`](../tools/verkstead.wxs), and all of it goes into the
profile: `%LOCALAPPDATA%\Programs\Verkstead`, the user's own Start menu, and
the user's own `PATH`, so that `verkstead ask` works in a terminal opened
afterwards. Per-user because the package is unsigned, and asking for
administrator would be an unsigned program asking for the machine. It runs on
Windows only: the WiX toolset's `candle` and `light` compile the package, and
MSVC and the Windows SDK build what goes in it.

### The sessions suite, and the machine under it

`crates/server/tests/sessions.rs` is the one suite that is really a hundred and
forty small servers, each running a real session in a real sandbox and judged on
wall clock. That makes it the one suite whose result depends on what else the
machine is doing, so it has two knobs of its own.

`VERKSTEAD_TEST_PACE` is a multiplier over everything time-shaped in it — the
budgets a session is judged idle, rescued and ended by, how long a wait gives
up after, and every window a test holds open to prove nothing happened. Unset
is `1.0`. CI sets `2`, because a two-core runner building the workspace
alongside the run cannot meet a developer's machine's budgets, and a session
descheduled past one is judged by the wrong rule and fails a test that is not
about the code. Raise it locally if the suite fails on a busy machine and passes
on a quiet one; the file's own `PACE` explains the rest.

Nothing needs setting for the concurrency: the suite caps how many fixtures stand
at once by itself, at twice the cores it can see.

```console
$ scripts/soak-sessions.sh        # ten runs, pinned to two cores, under a build loop
```

The soak is how a change to that suite is shown to hold. Running it again was
never how its flakes reproduced — each loaded run failed a different test and
every one of them passed alone — so the bar is ten in a row under load rather
than one green run. It takes about fifty minutes, which is why nothing runs it
for you.

**Windows runs an arm of its own**, because that suite is `cfg(unix)`: it
stands on a mount namespace, a `/bin/sh` probe and a `stty`, and a Windows
session has none of the three. `crates/server/tests/sessions_windows.rs` asks a
dozen of the same questions of the other machine — a grilling started by
pressing the button, run on a pseudoconsole in a profile of the Conversation's
own, with a PowerShell script where claude goes — and
`crates/server/tests/terminal_windows.rs` is the console's own pair beside
`terminal.rs`, asking `mode con` what `stty` is asked there. Both are
`cfg(windows)`, so a Linux or a Mac checkout compiles neither: the
`windows-2025` job is where they run, and it installs an `sccache` on the
runner for the three of them that are about the build cache — a Compile Server
coming up as the session account, a session handed a `RUSTC_WRAPPER` it can
run, and a Rust repository's session really compiling through that server. The
last of those wants a rustup-installed Rust on the runner as well, which is what
a Windows runner has: it runs the machine's own `rustc`, through the rustup home
a session is granted.

**And a Windows checkout needs the session account, once.** Every suite that
starts a session starts it *as* that account — `sessions_windows.rs`, the two
boundary suites, `account_windows.rs`, `launcher_windows.rs`, and the
`sandbox::granting::writing` tests inside the server crate — and the wizard's
sandbox row at the foot of `account_windows.rs` asks whether there is one at
all. None of them can make an account, that being an administrator's call. So a
machine that has never run the verb fails there with the line naming it rather
than passing quietly, and the verb is run once from an elevated terminal — which
is also what the wizard's first step now does for whoever is at the app:

```console
$ verkstead session-account create   # elevated, once per Data Directory
$ verkstead session-account remove   # elevated, and the way to undo it
```

**It is the account of the machine's *own* Data Directory that they use.** An
account's name is a fingerprint of the Data Directory it belongs to, and every
one of those suites runs against a temporary one — so the account they would be
named after is one no verb was ever run for. What they run as instead is the
account the human really has, said outright on the `Homes` a sandbox is built
against and with its password copied into the fixture's own `secrets.yaml`. Run
the verb with no `--data-dir`, which is the default the suites resolve.

The `windows-2025` job runs the same verb in a step of its own, a runner being
elevated already. Nothing else about a test run is privileged, here or there.

**And `sessions_windows.rs` wants the workspace built first**, which
`cargo test` does not do for it: a session comes up on a console made on the far
side of the boundary by `verkstead session-launcher`, so the image a description
names has to be a real `verkstead.exe` rather than the test harness's own —
and a server-crate test has no `CARGO_BIN_EXE_verkstead` to read. It looks for
one beside its own binary, in `target/debug`, and says so where there is none.
`cargo build --workspace --all-targets` puts it there, which is what the
`windows-2025` job does before it runs anything.


And in `web/`, which is the Solid viewer
([ADR 0003](adr/0003-solid-spa-viewer.md)):

```console
$ pnpm install
$ pnpm dev                # the viewer on :5173, /api proxied to the server
$ pnpm test               # the vitest suite
$ pnpm typecheck          # tsc, which the tests do not run
$ pnpm lint               # the wall around the query hook, and nothing else
$ pnpm build              # static assets into web/dist, and the share into web/dist-share
```

`pnpm dev` serves the viewer alone and proxies everything under `/api` to a
server on its usual `127.0.0.1:8422`, so the two run side by side in two
terminals and the browser sees one origin. The proxy is a development thing
only: the built assets are served by the server itself, out of the same binary.

Which is `pnpm build`'s output, embedded by rust-embed. A release build compiles
it in; a debug build reads it off disk per request, so a `cargo run -p
verkstead-cli -- serve` serves whatever `pnpm build` last wrote without a
recompile — and a checkout that has never built the viewer still builds the
server, which then says so on every page instead of serving one.

`pnpm build` writes three times, out of the same sources. The site goes to
`web/dist` as it always did; the **share** goes to `web/dist-share` as one HTML
file with its script and its stylesheets inlined
([`vite.share.config.ts`](../web/vite.share.config.ts)), which is the template
the server writes a Conversation into and hands over as a download — see
[`crates/server/src/sharing.rs`](../crates/server/src/sharing.rs). Both are
embedded the same way and both are `allow_missing`, so a checkout that has built
neither still builds the server; ask it for a share and it says the build is not
in the binary. That config refuses to write a document that still points at a
file beside it, which is what makes *no external requests* a property of the
build rather than something to remember.

The third is mermaid, on its own, into `web/dist-share/mermaid.js`
([`vite.mermaid.config.ts`](../web/vite.mermaid.config.ts)). It is the one thing
a Set's page draws for itself and it is three megabytes, so it is the one thing
a share does not carry as a matter of course: the share build aliases the
package to a stub that reaches for whatever the *document* is holding, and the
server writes the library into a second slot only where something in the record
has a Diagram on it — a Set's Preface or a Commit Summary alike. A Conversation
nobody drew a picture in stays the size of its own record.

The same file is what a **publish** puts in a secret gist — see
[`crates/server/src/publishing.rs`](../crates/server/src/publishing.rs), where
the API makes the gist and git fills it, because the Gists API will not take a
file this size. So a publish wants the share build too, and a token with the
`gist` scope on it.

`cargo test` covers the round trip in-process. `nix flake check` runs the
viewer's vitest suite from the pinned pnpm and node, and boots a VM with the
NixOS module enabled to put a Question Set through it again, for the sake of
everything the module wraps around that round trip: a unit that starts itself
at boot, the Data Directory systemd hands over, a database that survives the
service being stopped and started under a waiting agent, a store-path binary
serving the viewer that was built into it, and the CLI on `PATH` with nothing
set in the environment. The VM needs a Linux host to boot the guest on, so on
macOS that half of the check is absent rather than failing.

The viewer's dependencies are fetched by a fixed-output derivation named by a
hash in [`nix/web.nix`](../nix/web.nix) — move `web/package.json` or
`web/pnpm-lock.yaml` and that hash has to move too, and nix will print the one
it wanted. That file both builds the viewer and, with `runTests` on, is the
`nix flake check` above, so the two cannot disagree about a lockfile.

`assets/` is vite's `publicDir`, copied verbatim into the site root: the web
manifest, the icons and the service worker. They cannot live under `/assets/`
with the hashed bundles — a service worker only controls the paths beneath the
one it was served from, so one under the bundles' directory could never show a
notification for `/sets/12`, and the manifest and the icons keep the names the
phone knows them by. Which is also why the server keeps everything under
`/assets/` for a year and revalidates everything outside it.

The worker itself does no caching; every list and every Set is read from live
SQLite, and a cached copy of one that has since been answered is worse to the
human than a failure to load.

The icons are all downscaled by the script above (using ImageMagick from the dev
shell) to the sizes the favicon, the manifest and iOS ask for. The smaller PNGs
are committed so a build needs nothing but cargo — re-render the artwork and
re-run the two cut scripts rather than touching them.

There is one piece of artwork, and every icon in the repository is a downscale
of it:

| Artwork | Cut into | Where it comes from |
| --- | --- | --- |
| `tools/hammer/verkstead-hammer.png` | `icon-32`, `icon-192`, `icon-512`, `apple-touch-icon`, and every icon under `packaging/` | A 1024 square rendered out of [`tools/hammer/verkstead-hammer.blend`](../tools/hammer/verkstead-hammer.blend) by [`tools/hammer/render.py`](../tools/hammer/render.py). The blend file is the mark's source of truth; the render is what the two cut scripts read. Blender comes from the machine rather than the dev shell — see below, where `uv` does too |

It sits in `tools/hammer/` rather than beside the icons it is cut into, because
`assets/` is vite's `publicDir`: everything under it is served at the site root
and, the viewer being embedded, carried inside every binary including the
headless CLI. Nothing serves the 1024 square — only the two cut scripts read
it, and they read it from the repository — so a copy of it in every binary
would be over 600 KB nobody ever asks for. It was under `assets/icons` while it was
the mark's source and had a claim to be the viewer's too; the blend file is
that now, and the render is an intermediate like the cut icons are.

The iOS icon is the only output with a field under it and the only one with a
margin: iOS ignores transparency and composites whatever it is given onto
black, so `tools/generate-icons.sh` flattens the render onto the chrome's own
`#21201e` — the manifest's `theme_color` and the document's `theme-color` tag,
so the tile reads as the app's rather than as a third colour — and draws it at
160 inside a 180 square, because iOS rounds the tile's corners itself and the
head and the handle both run to the edge of the artwork. That colour is a
literal in the script; move `theme_color` and it has to move too.

The manifest asks for `any` rather than `any maskable`: the artwork runs to the
edges of its square, and a launcher masking it to a circle would cut the head
and the handle's end off. Art with a margin inside it could claim `maskable`
back.

A session working on that artwork drives Blender over the MCP rather than by
writing scripts at it, and two committed files are what put that server in
front of it. [`.mcp.json`](../.mcp.json) names it — `uvx blender-mcp`, with
`DISABLE_TELEMETRY=1` because the package phones home unless it is told not to,
and the loopback address and port it dials at the other end — and
[`.claude/settings.json`](../.claude/settings.json) approves that one server by
name, an unapproved project server being one a session is prompted about and a
session having nobody to prompt. They are approved by name rather than with the
blanket key, so a server somebody adds later is not approved by accident.

Both are committed for good. A session reads them when it starts and never
again, and Verkstead hands a sandboxed session a copy of the account's
configuration with its own servers taken out
([ADR 0011](adr/0011-agent-backends.md)), so the repository's own files are
the only route: drop them and the next session that has to re-render the icon
has no Blender.

**Two tools here come from the machine rather than the dev shell**, and
`flake.nix` carries neither: `blender`, which renders, and `uv`, which
`.mcp.json` runs the server through as `uvx` and which
[`tools/hammer/serve.py`](../tools/hammer/serve.py) shells out to again for the
bundled addon. A `nix develop` with Blender installed and no `uv` gets an MCP
server that never connects and a `serve.py` that dies in `subprocess.run` —
which is a confusing way to find out, so it is said here rather than left to be
discovered. Neither is in the shell because neither is wanted by a build or a
test: the icons are committed, and only a session re-cutting the artwork needs
either.

The server connects to Blender lazily, on the first tool call, and what it
connects to is [`tools/hammer/serve.py`](../tools/hammer/serve.py) — the addon
the same package bundles, running inside a Blender and listening on that port.
Start it before the first tool call and leave it running:

```console
$ blender -b --python tools/hammer/serve.py &   # 9876, unless -- --port says otherwise
$ kill %1                                       # closes the socket and unregisters the addon
```

It is a script rather than an addon installed into a Blender profile because
Blender here has no window and the bundled addon refuses to start without one —
its commands run from a timer callback and a background Blender runs no timers.
The script supplies that timer itself; its own header says the rest, including
why the virtual display the addon suggests is not an option in a Sandbox.

`packaging/` is the second tree of generated assets, and it sits outside
`assets/` deliberately. Everything under that directory is `publicDir` — served
at the web root and, because the viewer is embedded, carried inside every binary
including the headless CLI — and a desktop entry and a launcher's icons are
neither the viewer's to serve nor the CLI's to hold. So the desktop packaging
gets a directory of its own: `net.tobico.Verkstead.desktop` and the hicolor icon
tree that `tools/build-appimage.sh` installs into the AppImage,
`net.tobico.Verkstead.icns` that `tools/build-macos-dmg.sh` puts in the app
bundle, and `net.tobico.Verkstead.ico`, which Windows wants twice:
`crates/desktop/build.rs` compiles it into the shim as a resource, nothing
installed beside an exe being what Alt-Tab and the taskbar draw it with, and
`tools/verkstead.wxs` names it again for the entry the msi leaves in Apps &
Features. It is written by
[`tools/generate-packaging.sh`](../tools/generate-packaging.sh) from the same
hammer, and committed for the same reason the viewer's icons are. That script
rewrites the whole directory from nothing on every run — so a size
that stops being generated stops being committed, and nothing under it is ever
edited by hand.

The icns is written by the script itself rather than by `iconutil`, which is a
Mac's: the format is a header and a PNG per icon slot, so it is generated in the
dev shell alongside everything else here rather than on the one platform that
reads it. The ico is `magick`'s own output, that tool being in the dev shell
and having no such restriction — and the sizes it is handed are those same
committed downscales, so every platform draws the same pixels.

The tests run the real server in-process, so the round trip they check is the
one an agent gets — including the quickstart above, whose example files
[`crates/cli/tests/ask.rs`](../crates/cli/tests/ask.rs) drives end to end, taking
the human's part over the API the viewer's **Submit** posts through.

`verkstead-render` is everything the server does to what an agent wrote before
it leaves: markdown to sanitized HTML, the Diff parsed and highlighted, and the
view types the viewer draws a Set from. It knows nothing of the store, the router or
the viewer, so it is the seam the browser never reaches across — everything past
it is HTML the viewer only has to put in the page.

Two things under `web/` are written by `cargo test` rather than by hand, and
both are committed so that the diff is the review. `web/src/api/types.ts` is
those view types as TypeScript, generated by ts-rs — the viewer imports them and
declares no shape of its own, so the two languages cannot come to disagree about
a field. `web/tests/fixtures/` holds a payload of each kind, rendered by the real
`/api/ui/` endpoints, which is what the vitest suite is fed: a component test
against a hand-written mock proves only that the mock and the component agree.

Every query in the viewer is made with `useReading` from `web/src/freshness.ts`
rather than with the hook underneath it. A Nudge invalidates what the page is
showing ([ADR 0009](adr/0009-scoped-nudges.md)), so each query has to say what a
re-read does to what is drawn: `freshness` names the key the re-read is merged
by, or says `"static"` where the payload cannot change. There is no default —
a query that says neither does not typecheck — and `web/eslint.config.js` is
the other half of the same rule, refusing the raw hook everywhere but the
wrapper's own module. That is the whole of what `pnpm lint` checks.

Which queries a Nudge invalidates is one table in `web/src/nudge.ts`, keyed by
the kind the server sent. The kinds themselves are a Rust enum in
`crates/schema/src/nudge.rs`, so adding one is two edits — the variant and the
announce site on the server, and a row in that table on the viewer. A kind with
no row falls back to reading everything, which is what an older page does
against a newer server; `web/tests/nudging.test.tsx` sweeps the generated
fixture and fails on a kind the table has forgotten. Nothing polls: what stands
behind a Nudge that never landed is the catch-up on reconnect and on the page
becoming visible again.
