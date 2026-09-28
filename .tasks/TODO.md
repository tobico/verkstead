# Shared Profiles

An Agent Profile from one device runs sessions on another. Each device keeps a
**mirror row** per member Profile it has heard of, marked with its home device
and the id it has there, so every pairing, every Repo's pairing memory and every
Conversation goes on holding a local Profile id. The pickers are cluster-wide
with the device drawn on the row, and every device's Profiles section lists
everyone's, an edit or a removal relayed to the home device.

Start on B under a Profile whose account lives on A, and B mirrors the login and
the configuration from A under its own Data Directory before launch, syncs what
A remembers of this Repo over with the paths rewritten for this machine, and
writes both back to A as the session ends. A Profile whose home holds no login
file, or whose harness is absent here, or whose home has stopped answering,
reads as broken on the row and Start under it is refused by name. Repos are
matched across devices by origin URL, then by name — the reading the path
rewrite needs here and transfer needs again.

Roadmap stage: [08: Shared Profiles](docs/roadmaps/cluster-mode/08-shared-profiles.md)

## Tasks

- [x] 01: Mirror rows and the cluster-wide picker — [details](01-mirror-rows-and-the-cluster-wide-picker.md)
- [ ] 02: Edits and removals relayed home — [details](02-edits-and-removals-relayed-home.md)
- [ ] 03: Repo matching across devices — [details](03-repo-matching-across-devices.md)
- [ ] 04: The account mirror, and a session away from home — [details](04-the-account-mirror.md)
- [ ] 05: The login written back home — [details](05-the-login-written-back-home.md)
- [ ] 06: Memory sync — [details](06-memory-sync.md)
- [ ] 07: Broken states and the refusals — [details](07-broken-states-and-the-refusals.md)
