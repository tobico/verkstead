# Keeping the Annex

The settings page gains a **Components** card whose pane lists *Linux
development (WSL)* as its one row: running or stopped, the version it runs, the
disk it takes, **Install** where there is none, and **Update**. A Keeper updated
to a new release re-pins its Annex to that release and rebuilds it at its next
start, before the boot, the row reading *updating* meanwhile; Update moves both
nixpkgs inputs and is refused while a session runs in the Annex, naming the
Conversation. Everything about the work rather than the machine is copied into
the Annex at every boot, and a distro unregistered by hand is forgotten at the
Keeper's next start, on every device.

It is what turns an Annex that was made once into one that is kept: the same
release as its Keeper, the Keeper's settings, an update the human can press,
and a box removed by hand noticed rather than left as a dead row
(ADR-0024, *The box on Windows*). Demonstrable end to end on Windows.

Roadmap stage: [04: Keeping the Annex](docs/roadmaps/wsl-annex/04-keeping-the-annex.md)

## Tasks

- [x] 01: The Components card and pane — [details](01-the-components-card-and-pane.md)
- [x] 02: Install from the pane — [details](02-install-from-the-pane.md)
- [x] 03: The re-pin at start — [details](03-the-re-pin-at-start.md)
- [x] 04: Update — [details](04-update.md)
- [x] 05: The settings copied at boot — [details](05-the-settings-copied-at-boot.md)
- [ ] 06: Forgetting a removed distro — [details](06-forgetting-a-removed-distro.md)
