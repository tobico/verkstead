# Open repo on an Annex

Open repo, on a Draft whose device is an Annex, first asks where the
repository is — on Windows or on Linux. The Windows side registers the path on
the Keeper and ends in a clone on the Annex, so a repository cloned on Windows
becomes a working Repo there with no terminal involved; the Linux side opens
on the home of the box's own login user — which becomes `verkstead` itself —
so a repository cloned by hand in the WSL shell can be opened and worked on
directly. Settled in the grilling of 2026-11-23; ADR-0024's *Open repo asks
where the repository is* carries the decisions, and CONTEXT.md's **Repo**,
**Clonable Repo** and **Annex** entries are already amended on this branch.

## Tasks

- [x] 01: The box logs in as verkstead — [details](01-the-box-logs-in-as-verkstead.md)
- [ ] 02: The choice on the card, and the Linux side — [details](02-the-choice-and-the-linux-side.md)
- [ ] 03: The Windows side — [details](03-the-windows-side.md)
