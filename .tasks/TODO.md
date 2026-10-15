# Making the Annex

On a Windows machine, a tick on the wizard's *Linux development (WSL)* row
is all it takes to get Linux development. Verkstead turns WSL on through the
UAC dialog and imports a NixOS-WSL distro of its own, named `Verkstead`, with
its disk under `%LOCALAPPDATA%\Verkstead\wsl`. It writes the distro's flake
and configuration and rebuilds it. The Keeper then boots it and links it by
the **Keeper's Join**, with the forward up, and nobody presses anything.

The result is an **Annex** in the compose page's device select, reading
*Linux (WSL)* with the Annex mark. A Conversation drafted onto it runs under
bwrap with a Profile mirrored from the Keeper, and commits as the Keeper's
git author. The Annex boots with its Keeper and stops when the Keeper stops.
Stage 04 adds the upkeep (Components, re-pin, Update, the settings copy).

Roadmap stage: [03: Making the Annex](docs/roadmaps/wsl-annex/03-making-the-annex.md)

## Tasks

- [ ] 01: The backend interface and the WSL probe — [details](01-the-backend-and-the-wsl-probe.md)
- [ ] 02: The wizard row and the elevated step — [details](02-the-wizard-row-and-the-elevated-step.md)
- [ ] 03: The flake and config Verkstead writes — [details](03-the-flake-and-config-verkstead-writes.md)
- [ ] 04: Making the Annex on the install screen — [details](04-making-the-annex-on-the-install-screen.md)
- [ ] 05: Boot, link and terminate with the Keeper — [details](05-boot-link-and-terminate-with-the-keeper.md)
- [ ] 06: A session in the Annex — [details](06-a-session-in-the-annex.md)
