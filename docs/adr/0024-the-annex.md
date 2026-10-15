# The Annex

Amends [ADR-0020](0020-cluster-mode.md): a device can make a device. A
Verkstead on one machine makes a second Verkstead under virtualization on the
same machine, runs it for as long as it runs itself, links it into the cluster
with no press from anybody, and keeps it — the **Annex**, kept by its
**Keeper**. The first Annex is a NixOS inside WSL on a Windows machine, which
is what makes Linux development on Windows a tick in the wizard; the shape is
written so that a Docker or Lima box on a Mac, or a Windows VM on a Linux
desktop, is the same thing with another backend. The point is one machine
working with several operating systems.

Decided in the grilling of 2026-10-14. The roadmap that builds it is
`docs/roadmaps/wsl-annex/`.

## The Keeper's server owns the Annex, and the desktop app changes nothing

The Brief put the desktop app in charge of starting and stopping the Linux
server. The wizard, the Devices list and the link are all the server's, and
the Electron app is a thin shell that starts one `verkstead.exe` and lends it
elevation through `--desktop` (ADR-0020, *Electron desktop*). So the server
owns the Annex whole — provisioning in the wizard's install step, boot and
terminate tied to its own start and stop, the link, and the upkeep — and the
app contributes exactly what it contributes today. Which is what lets the
wizard row be ticked from a phone, and a headless `verkstead serve` on
Windows keep an Annex as the app's sidecar does.

**The app owning it was rejected**: the wizard would have had to reach the
app over the preload bridge, which exists in the app's own window and nowhere
else, and two servers would have been scripted from TypeScript. **Splitting
it** — the app booting the distro beside its sidecar, the server doing the
rest — was rejected for putting the lifecycle in one process and the knowledge
of what is being booted in another.

**An Annex runs only while its Keeper runs.** The Keeper boots it in the
background at its own start and terminates it at its own stop, and never
waits on it: a workbench opens on the Keeper while the Annex is still coming
up, and the Annex's rows read *unreachable* until it answers. An Annex that
will not boot, or will not rebuild, is a Notice on the Timeline and a line on
its row in **Components**, with the Keeper carrying on.

## The Keeper's Join

A join cannot complete without Allow on the far end (ADR-0020, *Linking*),
and nothing about an Annex should ask for one: its Keeper made it, and owns
the data directory its certificate sits in. So an Annex is linked by a
**Keeper's Join**, which is distinct from a regular join on purpose rather
than a regular join driven from both sides:

- **The Keeper writes its own Device Id and fingerprint into the Annex's
  config** before the Annex first starts. An Annex is told it is kept; it does
  not find out.
- **The Annex accepts a join that presents that fingerprint without holding a
  request and without a modal**, and records the Keeper as a member marked as
  its Keeper. Any other join is held and asked about as before.
- **The Keeper reads the Annex's `device.pem` out of its data directory and
  pins it before it dials.** That is the same trust a human comparing two
  fingerprints by eye has, done by the one party that can read both files.
- **The membership is the Keeper's.** The mark saying *Annex of that device*
  travels with the roster — on the exchange and on every announcement — so no
  device offers Unlink on an Annex's row, and an Annex never unlinks its
  Keeper. What removes an Annex is removing the box, which its Keeper notices.

Driving the ordinary join over both workbench keys — the Keeper posting the
join and then pressing Allow through the Annex's own API — was the
recommendation and was rejected by the human: an Annex is not a device that
happened to be linked, and the record should say so.

**The Annex never enters Onboarding Mode.** Its objective would be unmet at
its first start, there being no Profile until the mirrors arrive; the mode
only redirects a viewer nobody opens, but a kept Annex is told so and skips
the verdict rather than carrying a mode that nothing will ever clear.

## The Keeper forwards for its Annex

A WSL has a NAT address only the Windows machine can dial, and the cluster is
symmetric: a third device relays to every member itself, and a Profile mirror
fetches its login from its home device before each launch. Three things ruled
out the easy answers — loopback is never advertised, a bare IP is always
dialled at 8423, and every successful dial overwrites a member's addresses
with what it advertises — so an Annex cannot be a member at `localhost`, and a
member at its NAT address is one nobody but the Keeper reaches.

So **the Keeper listens on a port of its own, 8424, and forwards raw TCP to
the Annex's peer listener**. TLS stays end to end on the Annex's own
certificate; the Keeper never terminates it and never reads a byte of it. And
**the Annex advertises its Keeper's addresses at that port** — asked from the
Keeper at each start rather than configured, because a laptop's addresses move
— so a third device records the Annex at an address it can reach, and the
Annex is a full member of the roster. The Annex dials out on its own: its
Keeper for announcements and account mirrors, and any member through the
Keeper's addresses as the roster says.

**The Annex listens on ports of its own** — its peer listener on 8425 and its
workbench on the loopback at 8426, never 8423 and 8422. Under WSL's default
NAT that changes nothing, but `networkingMode=mirrored` in `.wslconfig` is a
per-user setting every distro shares, and under it the Annex has the Keeper's
interfaces and the Keeper's loopback: the ports would collide, and there would
be no NAT address to forward to. On ports of its own the Annex works under
either, and the forward points at the Annex's NAT address under NAT and at the
loopback under mirrored — which mode it is being the backend's probe to say.

**Keeping the Annex private to its Keeper** — not announced, its work shown
only in the Keeper's workbench — was rejected for making a Mac blind to half
the cluster. **Tailscale inside the box** was rejected for the tailnet login
it would cost every Annex.

An Annex is never advertised over mDNS and never discovered: its address is
its Keeper's.

## The Keeper and its Annex reach each other directly

*Amended 2026-10-15, after the review of stage 01.* Advertising the Keeper's
addresses serves a third device, but as written the Keeper and its Annex used
the same addresses to reach **each other**: every dial overwrites a member's
addresses with what it advertises, so the Keeper held its Annex at its own
LAN and tailnet addresses at 8424 — dialling itself through its own forward —
and the Annex held its Keeper at the addresses the Keeper's Join carried.
Offline the Keeper advertises nothing, after a network move it advertises
somewhere else, and either way the pair lost each other until the Keeper next
started. The one road between them that does not move with the Keeper's
network was the forward's own target, and no dial used it.

So **each holds the other at a direct address**, a column of the member row
that this device writes from what it knows of the machine and that no
refresh touches — the far end's advertisement is the far end's word, and this
is not. **Every dial to a member tries the direct address first**, then the
advertised ones, an advertised address that is the same place not tried
twice. Every other member has none.

- **The Keeper holds its Annex at the address the forward points at**: the
  Annex's NAT address under NAT, the loopback at the Annex's peer port under
  mirrored networking — the address the linking operation is given, written
  as it points the forward.
- **The Annex holds its Keeper where the Keeper says**, which the Keeper hands
  over in `kept_by` as `at` beside its id and fingerprint, and the Annex
  records from its own config when it takes the Keeper's Join. Under NAT that
  is the Windows host's side of WSL's network at the Keeper's peer port, under
  mirrored the loopback at it — the same probe that says where the forward
  points is the one that says this, and it is the Keeper's. **The Annex
  working it out** from its default route was rejected: it cannot tell NAT
  from mirrored, does not know the port its Keeper listens on, and would be a
  second reading of one fact the Keeper already has. And `kept_by` is
  rewritten at every link, so a host address that moved across a reboot is
  right again at the next one.
- **The Annex follows its Keeper at each reading**, as it already asked: the
  ask goes down the direct address now, so a move is advertised from the
  first reading after the answer's hold — five seconds — with nothing
  restarted. **The Keeper re-telling its Annex when its addresses change** was
  rejected as a second mechanism for what the reading already does.
- **A third device is told at the next link**, which re-reads the Annex before
  it announces it, and every start runs. Between two, a member holding a
  Keeper's old addresses holds its Annex's old addresses too, which is the
  ordinary cluster's lot when a laptop moves rather than an Annex's: the
  tailnet half of what is advertised is the half that does not.
- **The direct address is the pair's alone.** It is not in an identity, an
  exchange, a roster or an announcement — the advertised list is what those
  carry — so no third device is ever handed the Annex's NAT address or its
  loopback.

## The box on Windows

**A NixOS-WSL distro of Verkstead's own**, imported as `Verkstead` with
`wsl --import`, its disk under `%LOCALAPPDATA%\Verkstead\wsl` — beside the
logs rather than under the Data Directory, because `%APPDATA%` roams and a
multi-gigabyte disk image must not. It is separate from any distro the human
has: Verkstead writes its configuration whole, and a distro somebody else
configures is not one Verkstead can keep.

**Verkstead writes the box's flake.** Its inputs are nixpkgs on the stable
channel the repository's own flake pins, a second nixpkgs on the unstable
channel for the four harnesses alone, NixOS-WSL at a pinned release, the
`verkstead` flake at the Keeper's own release tag for the NixOS module, and
the release binary of that same version as a locked non-flake input. The
last of those is why nothing fetches by `nix/release.json`: the manifest is
committed *after* the tag, so a tag's own tree names the previous release,
where a flake input pointing at the asset locks its hash itself. **The Annex
runs the release its Keeper runs**, and a Keeper that finds the Annex pinned
to another version re-pins and rebuilds at its own start, before it boots
the Annex; nixpkgs and NixOS-WSL move only on **Update**.

Everything a session needs is declared in that configuration, because on
NixOS the wizard installs nothing: bwrap, git, gh, nix, sccache, and
`claude-code`, `codex`, `opencode` and `grok` from the unstable input where
nixpkgs has them. **Harnesses from nixpkgs rather than the vendors'
installers**: a package there lags the vendor by days where Ubuntu's lagged
by months, and Update moves them with everything else. **Building from
source inside the box** was rejected for the toolchain and the twenty minutes
per update; **shipping the configuration inside the msi** was rejected
because the msi's size is the download the human minds, and a later
download for an optional feature is fine.

**Repos live in `/srv/repos`**, bound through the module's `paths`, and are
browsable from Explorer at `\\wsl$\Verkstead\srv\repos`. Working on `/mnt/c`
was rejected again for ADR-0014's reasons.

**The Annex has no settings of its own.** The settings page is one device's
own and has no device select, so nothing in the UI reaches an Annex's; its
Keeper copies in, at every boot, everything that is about the work rather
than the machine — the git author, the GitHub token, the Instructions, the
MCP servers with their header values, the at-once limit and Cleanup. An MCP
server's header names are in `config.yaml` and their values in
`secrets.yaml`, so both files are copied from: a server copied without its
values is one declared in the Annex and refused by its far end. What stays the Annex's is what
is about its machine: its Repos, its Members, its sandbox binds.

**The wizard row reads *Linux development (WSL)***, on the first step on
Windows, unticked by default — it is a multi-gigabyte download and may need
a restart. Ticked, the elevated step turns WSL on (`wsl --install
--no-distribution`) and adds a firewall rule for the peer port on the WSL
adapter, which Windows Defender treats as a public network; a restart that
is needed lands on the hint screen as any missing row does, and the import,
the flake and the first rebuild run as the user with the install screen's
progress bar. A machine whose virtualization is off says so on the row.

**Components** is a new settings card, and the Annex's row is its first: the
status (running or stopped, version, disk use), **Install** for a machine
that skipped the wizard row, and **Update**. Update moves nixpkgs too, and
is refused while a session runs in the Annex, naming the Conversation — a
rebuild restarts the Annex's server, and every session in it with it. There
is no Stop, no Start and no Remove: a distro removed by hand
(`wsl --unregister Verkstead`) is forgotten at the Keeper's next start, its
member row gone and the Components row reading not installed. The card is
where installing more later lives too — a harness added after setup — which
is why it is *Components* rather than *Annex*.

**On the Devices list the Annex is a row like any other** — its Keeper's
hostname, which the written configuration sets as `networking.hostName`
(NixOS-WSL takes the distro's name from it, and its default is `nixos`), and
*Linux (WSL)* — wearing the
Annex mark and no Unlink; the press on it opens Components.

## Clonable repos

An Annex starts with no repository in it, and a cluster has always had the
same gap: a Conversation drafted onto another device needs the repository
*there*, and the Preflight's *no match* sent somebody to Open repo on that
machine. Across a cluster a Repo is matched by its origin URL (ADR-0020,
*Shared Profiles*), which is what makes a clone the same repository — so the
cluster already knows what could be cloned where.

**The Repo dropdown gains a second group after the device's own**: the
repositories other members have registered by origin that the named device
lacks. Picking one clones it on that device, into the parent **Create repo**
remembers there — asked for where none is remembered, and preset to
`/srv/repos` on an Annex — as the **Author** the member's Repo works as,
fetched with that Author's token (ADR-0025), registers it as that Author,
and it becomes the draft's Repo. **The Transfer dialog's *no match* offers
the same clone** rather than pointing at Open repo. A *Clone repo* row taking
a URL was the recommendation and was rejected for a list: the cluster knows
the origins, so nobody should type one.

## Consequences

- A **Keeper** is a device with at least one Annex, and an **Annex** is a
  device with exactly one Keeper; every other device is neither. The roster
  carries the relation, and the Devices list draws it.
- The peer listener gains a sibling on 8424 that forwards and never
  terminates TLS. The NixOS module gains nothing for it: an Annex's Keeper is
  never a NixOS daemon in the first version, and a forward is a Keeper's.
- `config.yaml` gains the Annex's `kept_by` — the Keeper's id and fingerprint,
  and where the Annex reaches it directly — the members table a direct
  address the Keeper and its Annex hold each other at, and the Annex's refusals name its Keeper by the name it last advertised.
- The Repo dropdown has three kinds of row: the device's own, the clonable,
  and the two that act.
- The Electron app is untouched, the msi does not grow, and `desktop/src`
  learns nothing about WSL.

## Considered options

- **The Electron app owning the Annex** — rejected: the bridge exists in the
  app's window only, and the wizard is the server's.
- **Driving the ordinary join from both ends** — rejected by the human: an
  Annex's membership is its Keeper's, and the record should say so.
- **An Annex private to its Keeper**, and **Tailscale inside the box** —
  rejected: the first blinds a Mac, the second costs a login per Annex.
- **Sessions on `/mnt/c`** — rejected, as ADR-0014 did.
- **Vendor installers inside the box**, **building from source**, **the
  configuration shipped in the msi** — rejected: nixpkgs is fresh enough,
  the toolchain is twenty minutes an update, and the msi is the download
  that matters.
- **Stop, Start and Remove on the Components row** — not wanted: status and
  Update are the pane, and a removed distro is noticed rather than offered.
- **A *Clone repo* row taking a URL** — rejected for a list of what the
  cluster already knows.
