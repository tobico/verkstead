# 04. Keeping the Annex

## Goal

The settings page has a **Components** card, and its pane lists *Linux
development (WSL)* as its one row: running or stopped, the version it runs,
the disk it takes, **Install** where there is none, and **Update**. A Keeper
updated to a new release finds the Annex pinned to the old one at its next
start, re-pins and rebuilds before booting it, the row reading *updating*
meanwhile. Update on the pane moves nixpkgs and NixOS-WSL too, and is refused
while a session runs in the Annex, naming the Conversation. The Keeper's
Instructions, MCP servers with their header values, at-once limit and
Cleanup are copied into the Annex at every boot. A distro unregistered by hand is forgotten at the
Keeper's next start: the member row is gone on every device and the
Components row reads not installed. The Devices row's press opens the pane.
Demonstrable end to end on Windows.

## Decisions in force

- **Components, not Annex** ([ADR-0024](../../adr/0024-the-annex.md), *The
  box on Windows*). The human named the card as the place that installing
  more later lives — a harness added after setup — so the Annex's row is its
  first and not its definition.
- **Status and Update are the pane.** No Stop, no Start, no Remove: the
  human chose the smallest pane, and a removed distro is noticed rather than
  offered.
- **The Annex runs the Keeper's release.** The re-pin is automatic at the
  Keeper's start and runs before the boot, so it never lands on live work;
  nixpkgs and NixOS-WSL move only on Update.
- **Update is refused while a session runs there**, naming the Conversation,
  because a rebuild restarts the Annex's server and every session with it.
  Writing it down to run once idle was the alternative and was not taken.
- **The Annex has no settings of its own.** Everything about the work rather
  than the machine is the Keeper's, copied at every boot: Instructions, MCP
  servers, the at-once limit and Cleanup. Its Authors are not copied — they
  reach the Annex as mirrors (ADR-0025). An MCP server's header names are in `config.yaml` and
  their values in `secrets.yaml` (`mcp_headers`), so the copy reads both: a
  server arriving without its values is declared in the Annex and refused by
  its far end. Its Repos, Members and sandbox binds stay its own.
- **Forgetting is not Unlink.** Stage 01 refuses Unlink on an Annex
  everywhere; the Keeper noticing the distro gone is the one path that drops
  the row, and it tells the roster the way an unlink does.
- **Install from the pane is the wizard's run**, the same plan and the same
  screens, for a machine that skipped the row.

## Proposed tasks (provisional)

1. **The Components card and pane** — the card on the settings page, the
   pane with the Annex's row: state read off the backend's probe, version
   off the flake's pin, disk off the image file; the Devices row's press
   landing here.
   - A machine with no Annex reads not installed with Install.
2. **Install from the pane** — the wizard's install run started from here,
   with its progress and its hint screen.
3. **The re-pin at start, and Update** — the version compared before boot,
   the flake re-pinned and rebuilt with the row reading *updating*; Update
   moving every input and rebuilding; the refusal while a session runs.
   - A Keeper one release ahead boots an Annex of its own version.
   - Update with a session running is refused naming the Conversation.
4. **The settings copied at boot** — Instructions, MCP servers, at-once and
   Cleanup written into the Annex's config before each boot, and the MCP
   servers' header values into its `secrets.yaml`.
   - A change on the Keeper's Instructions pane reaches the next session in
     the Annex after a restart.
   - An MCP server declared on the Keeper with an auth header is reached
     from a session in the Annex with that header sent.
5. **Forgetting a removed distro** — the probe at start finding no distro,
   the member row dropped and announced gone, the Components row reset.

## Re-verify at start

- Assumes stage 03 landed: the backend, the flake writer and the boot with
  the Keeper.
- Assumes the settings page is a list of cards in
  `web/src/settings/SettingsPage.tsx`, each with a pane, and the Desktop
  card is the model for one that reads state off the machine.
- Assumes the install run's state is the in-memory `Installer` in
  `crates/server/src/onboarding/install.rs` and can be started outside the
  wizard.
- Assumes Instructions, MCP servers, at-once and Cleanup are in
  `config.yaml`, the MCP servers' header values in `secrets.yaml` as
  `mcp_headers` (see `crates/server/src/settings.rs`), and nothing else of
  them is machine-bound.
- Assumes the unlink broadcast in `peer/unlinking.rs` is reusable for a
  Keeper telling the roster an Annex is gone.
