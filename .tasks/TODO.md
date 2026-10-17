# One list across the cluster

Every device of a cluster draws one list of Authors. An Author is at home on the
device whose `config.yaml` and `secrets.yaml` hold it, and every other device
keeps a mirror row of it — refreshed over the Relay on the Profiles' schedule —
and a copy of its token in its own `secrets.yaml` under the uuid. A Repo on a
member device can be pointed at a mirror, and its sessions and Verkstead's own
git and `gh` on it commit and push as that Author from the copy.

An edit of a mirror is relayed to its home device as that device's own edit,
verification included; a removal checks the home device's Repos only; a
Transfer onto a device whose Repo points at another Author is refused by the
Preflight naming both; and Open repo and Create repo on a member device offer
that device's one list.

Roadmap stage: [03: One list across the cluster](docs/roadmaps/authors/03-one-list-across-the-cluster.md)

## Tasks

- [x] 01: Mirror rows and seeing the one list — [details](01-mirror-rows.md)
- [x] 02: A Repo pointed at a mirror — [details](02-a-repo-pointed-at-a-mirror.md)
- [x] 03: The token copy — [details](03-the-token-copy.md)
- [x] 04: Edits, verification and removal relayed home — [details](04-edits-relayed-home.md)
- [x] 05: Open repo and Create repo on a member offer its one list — [details](05-member-pickers.md)
- [ ] 06: The preflight compares Authors — [details](06-preflight-compares-authors.md)
