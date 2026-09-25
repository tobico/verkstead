# Cluster mode

Amends [ADR-0015](0015-open-boundary-and-workbench-key.md): a Verkstead is no
longer alone. Several servers on a human's machines **link** into a cluster,
and the workbench of any one of them drives all of them — every Conversation
in one sidebar, a draft started on whichever device should do the work, an
Agent Profile from one machine run on another, and a running Conversation
moved between machines, by the human or by the agent itself when the work
needs another platform.

Decided in the grilling of 2026-09-25. The roadmap that builds it is
`docs/roadmaps/cluster-mode/`. The setups it is for: a laptop as the
interface with the work on a desktop; a desktop balancing work across two
servers; a Windows machine driving a WSL; a Linux desktop as the interface and
main server, with a Windows VM and a Mac transferred to when the work is that
platform's.

## A device is an id and a certificate

Nothing on the wire said which install answered (ADR-0015). Now each server
invents itself an **id** — random, short enough to sit in a URL — and a
self-signed **certificate**, both at first start, kept in the Data Directory
beside `workbench.key` and read back at every start after. The id is what
every record and URL names a device by; the tailnet node name was rejected
because it is gone the moment the machine leaves the tailnet, and the hostname
because two machines can share one. The **name** a device is shown under is
its hostname, read at each start, with an icon for its OS — nothing to type,
and the name the Brief called for. A WSL is detected from its kernel release
and read as *Linux (WSL)*, because a Windows machine and its WSL share a
hostname and the OS is what tells them apart.

## The peer listener, and mutual TLS

Devices talk over a **listener of their own**: TLS, every interface, port 8423
by default (`--peer-listen`, and an option in the NixOS module). Every call a
device makes to another presents its own certificate, and a link *is* the two
fingerprints each side holds — no bearer token, nothing to store beside the
certificates, and the handshake is the whole of verifying a call.

The workbench listener is untouched: loopback, `tailscale serve` in front of
it, plain HTTP to the browser. Two things ruled out sharing it. The served
address carries Tailscale's certificate rather than ours, so a link pinned on
a fingerprint could never go through it; and the workbench port speaks plain
HTTP to the browser and the serve alike. A listener that sniffed the first
byte for a TLS handshake was considered and rejected as a trick where a port
would do. Plain HTTP with a token per pair — the network as the perimeter, as
it was for the workbench — was the cheaper choice and was rejected by the
human: the link will carry Profile logins, and an mDNS-found peer is on a LAN
that may not be theirs alone.

Over Tailscale a peer is dialled on its tailnet IP at that port, encrypted by
WireGuard underneath TLS. The Conversation-scoped session API stays outside
the key gate and outside this listener both: it answers the loopback and the
pipe, which is all a session ever dials.

## A cluster is a membership

Linking A to B does not make a pair; it makes A a **member** of B's cluster.
Pairwise links were rejected because *control all of them through any one*
would then hold only where the human had linked every pair — N times N minus
one over two presses — and a transfer from B to C pressed on A would need B
and C linked besides. Under a membership, one confirmation joins the newcomer
to everyone: the device it joined through hands the newcomer every member's
identity, addresses and certificate, and **announces the newcomer to each of
those members over its own link**, which is a verified one already. Every
device's Devices list reads the same.

The announcement is the introducer's to make rather than the newcomer's,
because the vouching has to be carried by something. A newcomer that introduced
itself would be a stranger asking a member to record it, and a member has no way
to tell that from anybody else who can reach its peer port: the confirmation on
the one device would be the cluster's only gate, and every other member would be
joinable without passing it. Announced by the introducer, the claim arrives over
a link the member has already verified, and the newcomer's own first call is an
ordinary one from a device that member now knows.

**The join.** A presses Add on B's address. A dials B's peer listener,
accepts whatever certificate B presents for this one call, and posts a join
request carrying its id, name, OS, addresses and certificate. B holds the
request for ten minutes and asks its human: a modal in every open workbench
of B, raised by a nudge, and a push notification to B's phones, showing A's
name, OS, address and certificate fingerprint. A draws the same fingerprint on
its pending row, *Waiting for confirmation on B*, with Cancel, so the two can
be compared by eye. On Allow, B dials A back at the addresses A gave, checks
the certificate it meets is the one in the request, and hands over its own
identity and certificate and every member's; A checks B's certificate is the
one it saw. B then announces A to each of its own members, over the link it
already holds to each, and every one of them records A. A deny or an expiry
reads on A's pending row and is dismissed.

**Addresses.** A laptop moves between the LAN and the tailnet and DHCP moves
everyone, so every device advertises all its addresses — tailnet name and IP,
LAN IPs — on every exchange, and a peer tries them in that order. The address
typed at link time is only the first one known.

**Unlink** removes a device from the cluster for everyone: every member drops
it and it is told to drop them all. Asked once first, as Remove on a Repo is.
A member that cannot be reached stays on the list dimmed, reading
*unreachable*, and Unlink still works on it.

## Discovery

Two sources, both read when the Devices pane opens. **mDNS** in-process with
the `mdns-sd` crate — advertising and browsing `_verkstead._tcp.local`, the
TXT record carrying id, name, OS and peer port — so there is no avahi or
Bonjour to depend on. **Tailscale** by reading the peer list from `tailscale
status --json` and probing each online peer's port 8423 for a device identity,
in parallel with a short timeout, each time the pane opens rather than on a
schedule. A discovered device is drawn with its name, OS icon, address and
where it was found, with one press to Add; members are left out. Windows plus
WSL is the case where discovery may not cross — WSL2 sits behind NAT unless
mirrored networking is on — and typing the address is what is left.

## The opened device relays

The web client is same-origin: relative paths, one `HttpOnly` cookie per
origin, a relative nudge stream, relative sockets. Rather than teach it N
origins — CORS and cross-origin cookies on every device, every device served
to the phone, a query cache keyed by server — **the device the browser opened
relays for the rest**. A member serves the whole of `/api/ui/` to another
member over the peer listener, authenticated by the handshake rather than the
key cookie, and the hub forwards a call to `/api/ui/devices/{device}/…`
verbatim and hands the answer back untouched: calls, the nudge stream, the
terminal and file-watcher sockets, multipart uploads. A member's workbench key
never leaves it. Remote Conversations live at
`/devices/{device}/conversations/{id}` with every leaf under it; local ones
keep their URLs, because this device is where most of the human's work is and
a device segment on every URL would say nothing.

The hub holds one nudge stream to each member, keeps that member's
Conversation list in memory, refreshes it on a nudge, and re-announces every
member nudge locally under the device — so the merged list is served from
memory, and a member that stops answering keeps its last rows, dimmed
*unreachable*, presses on them refused by name. Fanning out on every load was
rejected: a round trip per device per refresh, and a member that is down would
stall the sidebar. A member's push news is relayed the same way, the hub
pushing it to its own phones with the device name leading the title. The hub's
own *Show archived conversations* switch governs the merged view.

## Ranks

The sidebar's order was a dense integer per row, the whole table rewritten on
every drag — a sentence that cannot be sent to two devices that each own part
of the list. Each device now keeps a **rank string** on its own Conversations,
given at creation above everything, and the hub merges by rank; a drag on the
merged list computes one new rank between its neighbours, whichever devices
they belong to, and writes it to the owning device alone. So the order reads
the same from every device. The keys are fractional-indexing strings — a key
between any two always exists, they grow only where one keeps inserting in one
spot, and there is no bucket to rebalance — over Jira-style lexorank with its
rebalance pass. A migration ranks every existing row in its present order,
unplaced ones on top newest first, and the *unplaced float to the top* rule
goes: a new Conversation is simply ranked above everything.

Once anything is linked, every row's second line reads OS icon, device, then
the repo — this device's own rows too, so the list reads as one — and the pane
header carries the device as a mark beside the branch, and the row read aloud
says it.

## Drafting on a device

A **device select** stands left of the Repo select on the compose page, drawn
only where another device exists, reading the device last picked in this
browser — remembered the way pane widths are — and this device until then.
Picking one makes the Repo select list that device's Repos and the pairings
prefill from it, and Start creates the Conversation there: the compose page's
replay goes through `/api/ui/devices/{device}/…` unchanged. A saved draft's
composer moves it too, by replaying it onto the other device and closing it
here. Inside the select's panel, under *May be transferred to*, a tick per
other device says where the agent may move the work; the device it was
drafted on is always permitted, so a session can go home. The same ticks stand
in the Transfer dialog afterwards, so the list can change at any time.

## Shared Profiles

A Profile is a thin row over an account directory on one machine, and a
session is given a Built Root made from that account. Across a cluster the
row travels and the account does not.

Each device keeps a **mirror row** per member Profile it has heard of, marked
with its home device and the id there and refreshed over the link, so
pairings, the Repo's pairing memory and every Conversation go on holding a
local Profile id and nothing that reads one changes. The pickers are
cluster-wide, the device name on the row; every device's Profiles section
lists everyone's, and an edit or a removal is relayed to the home device.

**Away from home**, device B launching under a Profile whose account lives on
A keeps a mirror of the account's login and configuration under its Data
Directory, fetched from A before each launch and written back to A as the
session ends — the way a Built Root is made from and written back to a local
account. The memory switch holds away from home too: this Repo's entries in
the memory store are synced over before launch and back after, their paths
rewritten to the machine they land on, which is what makes the harness's own
resume possible after a transfer. A login is an OAuth pair the harness
refreshes, so two machines refreshing one login at once may sign one of them
out; that is accepted, last write winning, a sign-out reading as the Profile
being broken there, the fix a login on its home device. Lending a Profile out
exclusively was rejected as a lock nobody asked for. A Profile whose home holds
no login file — a Claude login in the macOS Keychain — cannot be used away and
the picker's row says so; a harness absent on the device reads as broken there
in the onboarding probe's own words, and Start is refused by name.

## Repos across devices

A Repo on B is the Repo on A when their `origin` URLs match, and by name where
neither has an origin. No match refuses a transfer by name, pointing at Open
repo on B. Choosing by hand every time was rejected as a press on every
transfer for an answer git already holds.

## Transfer

Almost nothing about a running Conversation is process-bound: a restart
already recomputes what ought to be running from the record and the branch and
starts it, and a transfer is that recompute on another machine.

**The branch travels as a git bundle** over the link, packed against what the
target says it already has, with a binary patch of tracked changes and every
untracked file git does not ignore; the target fetches the bundle, cuts the
worktree on the branch and applies the patch — and the same for every
read-write companion. Ignored files stay behind and the target builds its own.
A WIP commit pushed through origin was rejected: it needs an origin, it lands
a commit there, and it is not how the human's working changes should travel.

**The record's slice** — Timeline, Sets and Responses, deferrals, steers,
stops, pull request and wrap-up bookkeeping, captures, transcripts,
attachments — is copied to the target under new local ids, its rank string
with it. The source **keeps its copy**, marked *transferred* and read-only.
Every copy carries a cluster-wide **birth key** — the device the Conversation
was drafted on and its id there — so the merged list draws only the live copy,
a transferred copy's URL redirects to it, and each device's own list shows its
transferred copies dimmed under *transferred to B*. A transfer back to a
device that still holds a copy replaces that copy wholesale by the live record
under its existing local id, so old links keep working and nothing is merged
by hand.

**The move runs when the session's turn ends**, as Stop does: the driver ends
the session, memory and login write back home, and then the slice, the bundle
and the patch go over. The Timeline says *Transferring to B* meanwhile. On
arrival the harness's own resume continues the agent's context where the
harness has one — Claude and Codex first — with a short note saying it now
runs on B with the worktree at its new path and the new ids of any Sets it had
open, the wait on them having gone with the process; Verkstead's Resume — a
fresh session re-primed from the record — is the fallback where the harness
has none or the log cannot be found. *Transfer to…* is a row on the
Conversation actions menu, opening a dialog with the device select and what
the preflight found missing, allowed from every state but Draft and Closed.

## The agent's call

`verkstead transfer <device>`, by the name or the id the prompt listed, an
ambiguous name refused naming both. It follows `verkstead done`'s pattern: a
request the driver acts on when the turn ends, never an action the route
takes. Before it returns it runs the preflight against the target — Repo
matched, harness present, device reachable — and exits non-zero saying what is
missing, so the agent can pick another device or ask the human. A device not
among the ticks is refused by name; the ticks are the human's consent, and no
confirmation follows — the Timeline says *Transferred to B at the session's
request*. The prompt names the permitted devices with their OS and the reason
to reach for the call — work the current platform cannot do — only where the
list is not empty, and the Guide carries a section on it. A transfer the human
presses ignores the list.
