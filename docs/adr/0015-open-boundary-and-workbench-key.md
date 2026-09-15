# The open boundary and the workbench key

Amends [ADR-0004](0004-single-binary-distribution.md): a bare binary no longer
comes up admitting nothing, because there is no longer anything to admit.

Designing a first-run wizard ([ADR-0016](0016-onboarding.md)) exposed two
things about the shape underneath it. The **Watched Path** — the one rule
about what Verkstead may touch — refused the account a Profile most naturally
names, the human's own login under `~`, so a wizard that detected accounts
would have found ones it could not save. And the boundary's real job, capping
what a session reaching the workbench's API over the loopback could register,
was one it only half did. This decision removes the boundary and closes the
hole it half-covered with a credential.

## The Watched Path goes

The Watched Path was one rule: a Repo is registered only from within one, an
Agent Profile's account must sit inside one, and the path browser is bounded
by one. It was there so that nothing outside a directory the human named could
be worked in or mounted.

It goes, everywhere. What keeps a session's reach to its own Conversation is
the Sandbox — the Worktree, the Repo's git directory, the account, and nothing
else of the machine — and none of that needs a boundary above it: the Sandbox
is composed from the Repo and the Profile the Conversation names, and those
are the whole of what it reaches. A boundary that bounded the *registering* of
those two things was a second fence around a first, and it made the first run
worse than the protection was worth.

So: no `--watched-path` flag and no `VERKSTEAD_WATCHED_PATHS`; no Paths
section beyond the Sandbox Configuration's binds; registering a Repo needs an
absolute path that is a git repository root, and nothing more; an account is
any directory of the shape its harness keeps; the path browser browses
anywhere the server can read, which it already did for the fields the boundary
said nothing about. The settings' resolution report — *the server cannot see
this* — goes with the section it stood on; the refusal a Repo or an account
the server cannot see gets is *missing*, which it already was.

**An unbounded browse opens at the server's own `HOME`, not at the filesystem
root.** The Watched Paths were what a bounded field started from, and nothing
else stands where they were: an unbounded browse asked with no path answers
the root, or the drive list on Windows, and every field the boundary used to
seed would now open several levels above anything the human meant. `HOME` is
where a repository and an account both actually live, so it is the one
directory worth starting at, and it is a starting point rather than a
boundary — the browse walks up out of it like any other.

**The NixOS module keeps an option, because the unit's namespace is not the
server's to widen.** `ProtectHome=tmpfs` and `ProtectSystem=strict` hide
everything the unit is not told to bind, so `watchedPaths` becomes **`paths`**:
the directories bound read-write into the unit, repositories and accounts
alike, with no minimum and no assertion. It has no Verkstead meaning — a Repo
or an account outside it is refused as *missing* like any path the server
cannot see, with a sentence naming the option on that install. `home` stays
bound read-only, so an account under it that a session must write is named in
`paths` as well.

Amended: **the refusal stays bare, and the documentation is what names the
option.** Nothing on the wire says which install a server is running as, so a
refusal carrying that sentence would have meant the unit telling the server what
kind of install it was — a mechanism built to hold one sentence. A Repo or an
account the unit was not told to bind is answered *missing* exactly as anywhere
else, and `adoption.md` is where somebody learns that `paths` is where to add
it.

**The cost, named.** A session's network is the host's own, and one listener
serves both the Conversation-scoped session API and the workbench's own
`/api/ui/`. The boundary capped what a session reaching the UI API could
register. That cap was already porous — the settings page could add a Watched
Path through the same API — and the next section is what actually closes it.

## The workbench key

The UI API and the workbench answer **401 without a cookie** the browser holds
and a session cannot. The cookie's value is one long-lived random secret, made
at first start and kept in the Data Directory, which no Sandbox mounts. The
link that sets it is the workbench's address with `?key=…`; opening it sets
the cookie and redirects to the same address without it. **Reset key** on the
Remote access pane re-issues the secret, which logs every device out.

Where the link is handed out is a fact about the install. The desktop tray's
**Open** opens the browser on it. The startup log line carries it, which is
the daemon's way. Neither a `verkstead` subcommand nor a wizard step: the first
would be a second place to print one line, and the second would put a step
nothing gates in front of somebody who has not yet seen a Conversation.

What stays open: the Share Viewer, which other people open from a gist link
and which reads nothing of the API; `/api/v1/health`; and the
Conversation-scoped session API, which is a session's own and scoped already.

**Remote access** is the settings pane the key makes necessary: a phone cannot
reach the workbench until it has the key, and the adoption docs' `tailscale
serve --bg 8422` was a command the human ran by hand. The pane holds three
things — a checkbox labelled **Allow remote access via Tailscale**, which runs
that command on and takes it off; a QR code of the login link, with a copyable
link beside it; and **Reset key**. Which of the four states — no `tailscale`,
a daemon that is not answering, up, or an answer this build could not read — the
machine is in is the card's own line above the pane, and the pane draws what the
machine said only where there is something to be done about it. **The checkbox
will not lock the page out**: a browser whose hostname is the served address's
is reading this over the very serve it would be turning off, so there the box is
disabled with a tooltip saying why. The client settles that out of the address
the reading already carries, the server seeing the tailnet and the loopback
arrive on one port.
`tailscale serve` from a non-root process needs that user set as the
operator: the desktop app runs the grant through the platform's graphical
sudo, the daemon shows the exact command and re-tries on the next press, and
the NixOS module sets the operator itself wherever `services.tailscale` is
enabled. The pane is pointed at by a **dismissable banner** on the
Conversation page at every grilling start until dismissed — kept on the
server, so one press on any device ends it everywhere — because the first
grilling working on its first Question Set is the first moment the human has
nothing to do at the desk.

## Considered Options

- **Keeping the Watched Path as an installation-only flag**, dropped from
  the settings. Rejected: two behaviours to document for a boundary the key
  replaces.
- **Copying a detected account into the Data Directory** and admitting
  accounts there. Rejected: a copy of a login that drifts from the real one.
- **Ticking an account adds its parent to the Watched Paths.** Rejected: for
  a Claude account the parent is `~`, which opens everything.
- **Sessions off the loopback** — a unix socket the way Windows gets a named
  pipe, `bwrap --unshare-net` with `pasta` for the outbound half, a seatbelt
  rule denying loopback. Costs a new Linux dependency the wizard would then
  probe for, changes all three sandboxes, and leaves Windows open until the
  AppContainer lands. The key covers all three platforms at once with nothing
  to install; the socket is not ruled out for later.
- **A short-lived key per QR code.** Rejected: one secret with a Reset press
  is one thing to reason about.
- **A `verkstead remote` subcommand** printing the link and a terminal QR.
  Rejected: the log line already carries it.
- **A wizard step for Remote access.** Rejected in favour of the banner.
