# Records are the cluster's

Amends [ADR-0020](0020-cluster-mode.md), whose *Shared Profiles* section made a
Profile a row at home on one device and a mirror everywhere else;
[ADR-0025](0025-a-repo-has-an-author.md), which gave an Author a home device
the same way and kept the record in `config.yaml`; and
[ADR-0021](0021-declared-mcp-servers.md), which kept a declared MCP server
there too. The Peer Listener, the Member Gate, the Relay and the membership
stand as ADR-0020 set them out; what changes is who a record belongs to.

Decided in the grilling of 2026-10-28 (Sets 1451 to 1453). The roadmap that
builds it is `docs/roadmaps/cluster-owned-records/`.

## What was there, and what it cost

An Agent Profile was at home on the device whose filesystem held its account,
an Author on the device whose `config.yaml` held it. Every other device kept a
**mirror row**, refreshed over the Relay by replacing what the home device last
said, and relayed every edit and removal back home. A cluster of four machines
— an always-on Linux desktop, and a Windows VM, a Linux laptop and a Mac
switched on when needed — therefore held **four unnamed Claude Profiles**, one
per `~/.claude`, and **four Authors** carrying the one human's name and email
under four uuids. Every pairing picker listed all four Profiles. And a Transfer
onto a device whose matching Repo pointed at *its* Author of that name was
refused by the Preflight, the two uuids being two Authors however alike their
fields, because each belonged to the machine that had made it.

Nothing in the codebase replicated a record. The nearest things were the
membership's owed announcements, which replace by key, and the login file,
whose *last write wins* ADR-0020 accepted for two machines refreshing one
OAuth pair.

## The decision

Each of the following was settled there, and the alternative named is what was
turned down.

- **A record is the cluster's, not a device's.** Agent Profiles, Authors,
  declared MCP servers and the Instructions text each carry a **uuid**, an
  `updated_at` and a **deletion stamp**, and every device holds every one of
  them. There is no home device, no mirror and nothing relayed home: an edit
  is a local write that replicates. Keeping the home model and fixing the two
  symptoms separately was not offered; the human asked for this.

- **Per record, the newest write wins**, the device id breaking a tie, and **a
  removal is final** — a tombstone is never overwritten by a later edit, so an
  edit made on a device that was apart when the removal happened stays
  removed. Letting the newest write win even over a removal, which would bring
  a removed record back, and per-field timestamps, which would let concurrent
  edits of different fields both survive at a timestamp per column, were both
  turned down for the simpler rule.

- **Every device serves its whole table** — rows and tombstones, whoever wrote
  them — on a route of the Peer Listener behind the Member Gate, and the others
  **pull and merge** at start, on the existing *moved* Nudge, when a member's
  Nudge stream is taken up again and when the membership moves. A pull is
  idempotent, so a missed Nudge costs nothing, and a record whose writer has
  gone still reaches a newcomer through whoever holds it. Tombstones are kept;
  the tables are tiny. Pushing changed rows to each member on write, queued as
  owed while a member is unreachable, was turned down.

- **Nothing merges.** Two Authors with one name and email, two unnamed
  Profiles of one harness, are two records until the human removes one. An
  automatic merge by a deterministic rule, and a Merge press offered over the
  pairs the page thinks are one, were both turned down: the human chose to
  clean up by hand. Two things make that bearable. **At the switch, a device
  that has members names each of its own unnamed Profiles after its hostname**,
  so four rows do not read *Claude — Default* four times over in a picker; a
  device in no cluster leaves its unnamed Profiles as they are. And **Remove on
  an Author that Repos point at names each Repo with its device and offers
  *Repoint them all to…*** over an Author picker, a Repo with a live
  Conversation refusing it by name as a repoint does today.

- **The uniqueness rules are enforced at the press** — one unnamed Profile per
  harness, no two named alike, no two live MCP servers of one name — against
  the whole table this device holds, and a pair that arrives by replication is
  kept and drawn, two alike, until one is renamed or removed. Dropping the
  rules was turned down.

- **Every reference to a Profile becomes the uuid**: the three pairing columns
  on a Conversation, the Repo's pairing memory, pending Steers, the models and
  homes tables, the account directory under the Data Directory, the peer routes,
  the `Login` Nudge and the viewer. Keeping the local integer id as the row key
  with the uuid beside it was offered and turned down.

- **A Profile's account is bound per device.** The record — name, harness,
  models, memory switch — is everyone's. The account paths are a **binding**
  of this device alone, replicated as a row per device so every card can say
  where the Profile is bound. A device without a binding runs its sessions from
  a **copy** under its Data Directory, built from the replicated login. Binding
  a device is a press on the card, *Use this machine's own account*, never
  automatic: the login at `~/.claude` on another machine may be a different
  account. Keeping the account on one device with only the row shared, and
  Verkstead importing the account once and owning it everywhere — which would
  part the Profile from the human's own harness login on that machine — were
  both turned down.

- **The login is replicated, newest write wins.** A bound device reads its
  account's login at each launch, at each session's ending and on the refresh
  schedule, and publishes it where it differs; every device keeps the newest
  it has heard of and an unbound one builds its copy from that. So a quiet
  device blocks no start. The risk is ADR-0020's, now across more copies: two
  devices refreshing one OAuth pair may sign one out, and the fix is a login
  on a bound device. Watching the account directory for changes was turned
  down in favour of the three moments.

- **Memory is unioned in and fanned out.** Before launch, this Repo's and this
  Worktree's entries are pulled from every reachable device, the newer file
  winning; after, what the session wrote is pushed to every reachable device.
  Nothing is deleted anywhere, as today. OpenCode's one database cannot be
  unioned and is taken newest whole. A memory-device field on the record,
  synced to and from as the home was, and no memory across devices at all,
  were turned down.

- **Authors, MCP servers and Instructions leave `config.yaml` for the
  database.** A record other devices write, carrying a timestamp and a
  tombstone, is a thing Verkstead *finds*, and `config.yaml` has only ever
  held what it is *told*. Tokens and header values stay in `secrets.yaml`,
  copied to every device over the gated route as Author tokens already are.
  Keeping them in `config.yaml` with a timestamp per entry and a deleted list,
  replication writing that file on every device, was turned down. The
  Instructions are one replicated text, newest wins, cleared being an empty
  text rather than a tombstone; a per-device text with a copy-to-all press was
  turned down.

- **A declared MCP server gets a uuid.** Its name stays what the agent and the
  attached chips see and is unique among live declarations across the cluster;
  deleting and declaring the same name again is a new record, which is what
  lets *a removal is final* hold without an exception for the one documented
  way to rename a server. Keying by name and letting a redeclaration beat the
  tombstone was turned down.

- **Removing an Author checks this device's Repos and every reachable
  member's**, naming the Repo and the device, and skips a member that is not
  answering, whose Repo refuses its next Start by name as today. Checking this
  device alone, and every member with an unreachable one blocking the removal,
  were turned down.

- **The Profile card carries a line per device** — bound, with the path, or
  *runs from a copy* — and the device mark on a Profile or Author row goes,
  there being no home to mark. Showing this device's binding alone, and nothing
  per device, were turned down.

- **New release only.** The sync talks to members on this release; a member
  too old to answer the route is a trouble line naming both versions, under
  ADR-0027's rule, and its Profiles and Authors are absent from the pickers
  until it updates. When it does, the mirror rows this device held of it are
  bridged to its new uuids by the home device and the id there, which the
  mirror already records. Keeping the old mirroring alive beside the new sync
  for members not yet updated was turned down.

- **Shaped so another table can follow.** The facility is one thing with four
  record kinds on it. Nothing else is brought along now; sandbox binds are
  paths on one machine and stay in `config.yaml`.

## What this does not change

A session is still given a Built Root made out of one account and no other,
and a Repo's sessions still carry that Repo's Author and none of the others
the device holds. The Transfer Preflight still compares Authors by uuid — which
is now a comparison that can come out equal. The Repo match across devices,
the Relay, the Member Gate and the membership are as ADR-0020 left them. The
onboarding wizard still asks for one Author and finds one account, and a
Verkstead in no cluster behaves as it did, its one unnamed Profile unnamed.
