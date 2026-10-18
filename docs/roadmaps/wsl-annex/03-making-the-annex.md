# 03. Making the Annex

## Goal

On a Windows machine with no WSL, the wizard's first step carries a row
reading *Linux development (WSL)*, unticked. Ticked, Next turns WSL on
through the UAC dialog and adds a firewall rule for the peer port on the WSL
adapter; a restart that is needed lands on the hint screen with the row's
own instruction, and the row ticks on its own once the machine is back and
WSL answers. Then, as the user and with the install screen's progress bar:
NixOS-WSL is downloaded at a pinned release, imported as `Verkstead` under
`%LOCALAPPDATA%\Verkstead\wsl`, given the flake and config Verkstead writes,
rebuilt, booted, and linked by the **Keeper's Join** with the forward up.
The compose page's device select lists *Linux (WSL)* with the Annex mark; a
Conversation drafted onto it runs its session under bwrap, under a Profile
mirrored from the Keeper, and commits as the Keeper's Author, mirrored to it
with its token's copy. Quitting the app terminates the distro; starting it boots the
Annex in the background and links it again. A machine whose virtualization
is off says so on the row and cannot tick it. Demonstrable end to end on
Windows.

## Decisions in force

- **The server owns it; the app changes nothing**
  ([ADR-0024](../../adr/0024-the-annex.md), *The Keeper's server owns the
  Annex*). Elevation is the wizard's own, through `--desktop`; a daemon with
  no way to ask sends the row to the hint screen as it does every other.
- **A backend behind an interface** — make, boot, terminate, run inside (as
  root and as the service user), read a file, and a probe of the machine's
  state (no WSL, needs a restart, virtualization off, distro present, distro
  running, and whether WSL's networking is NAT or mirrored) — with WSL the one implementation. The human wants a Docker or
  Lima box on a Mac and a Windows VM on a Linux desktop behind the same
  shape later; nothing of either is written now.
- **A distro of Verkstead's own**, named `Verkstead`, disk under
  `%LOCALAPPDATA%\Verkstead\wsl`, never a distro the human has.
- **The flake Verkstead writes** (ADR-0024, *The box on Windows*): nixpkgs
  on the channel `flake.nix` pins, an unstable nixpkgs for the harnesses
  alone, NixOS-WSL pinned, the `verkstead` flake at the Keeper's own release
  tag for its NixOS module, and the release binary of that version as a
  locked non-flake input — never `nix/release.json`, which a tag's tree
  names the previous release in. A development build names a binary copied
  in instead, through an override the brief's re-verify list names.
- **What the configuration declares**: the module with `listen` on the
  loopback at 8426, `peerListen` on every interface at 8425 — ports of the
  Annex's own, never the Keeper's 8422 and 8423, because under WSL's mirrored
  networking the Annex shares the Keeper's loopback and interfaces (ADR-0024,
  *The Keeper forwards for its Annex*) — `paths = [ "/srv/repos" ]`,
  `networking.hostName` set to the Keeper's hostname so the Devices row
  reads as the same machine rather than NixOS's default `nixos`,
  bwrap, git, gh, nix, sccache, and `claude-code`, `codex`, `opencode` and
  `grok` from the unstable input where nixpkgs has them; `kept_by` with the
  Keeper's id and fingerprint in `config.yaml`, and no Author: the Keeper's
  reach the Annex as mirrors (ADR-0025). The rest of the settings are stage
  04's.
- **Repos in `/srv/repos`**; the remembered parent for Create repo and for
  stage 02's clone on the Annex is preset to it.
- **Boot with the Keeper, terminate with it.** The Keeper's startup boots the
  distro in the background and never waits; once the Annex answers, the
  Keeper reads its address — the NAT address under NAT, the loopback under
  mirrored, as the probe says — points the forward at it and runs stage 01's
  linking operation again, which is idempotent. **The linking operation takes
  a second address from the probe too** (ADR-0024, *The Keeper and its Annex
  reach each other directly*): `keeper_at`, where the Annex reaches the
  Keeper — the Windows host's side of WSL's network at the Keeper's peer port
  under NAT, the loopback at it under mirrored — which goes into `kept_by` as
  `at`. The two are what the pair dial each other at whatever the Keeper's own
  network does, so both are read afresh at every boot: a NAT gateway that
  moved across a reboot is right again at the next link. The Keeper's shutdown
  terminates the distro. An Annex that will not boot is a Notice and a line
  on its row.
- **The elevated step**: `wsl --install --no-distribution`, `wsl --update`,
  and one firewall rule for the Keeper's peer port on the WSL adapter —
  Windows Defender treats that adapter as a public network, and the Annex
  dials its Keeper across it. The hint screen carries the restart.
- **Unticked by default**, the row's words *Linux development (WSL)* the
  human's own.

## Proposed tasks (provisional)

1. **The backend interface and the WSL probe** — the trait and the
   `wsl.exe` implementation of make, boot, terminate, run inside, read file;
   the probe's five answers and the networking mode.
   - Unit-tested over a stub `wsl.exe` the way the install suite stubs
     package managers; the probe reads each state off the stub.
2. **The wizard row and the elevated step** — the row on Windows, the plan
   entries for `wsl --install`, `wsl --update` and the firewall rule, the
   hint-screen instruction for the restart, the row ticking on its own once
   WSL answers.
   - A machine with virtualization off draws the row refused with the reason.
3. **The image, the import and the flake** — the pinned NixOS-WSL download
   into the disk directory, `wsl --import`, the flake and the two YAML files
   written in, `nixos-rebuild switch` with its output on the install screen.
   - The written flake pins the Keeper's own version and evaluates.
   - The booted Annex's identity answers the Keeper's hostname.
4. **Boot, link and terminate with the Keeper** — the background boot at
   startup, the address read, the forward pointed, stage 01's linking run;
   the terminate at shutdown; the Notice on a failure.
   - After a restart of the Keeper, Devices lists the Annex reachable within
     the boot's time and nothing was pressed — under NAT and under
     `networkingMode=mirrored` both.
   - The link names the Annex's address and the Keeper's as the probe read
     them, and the Annex stays reachable with the Keeper offline.
5. **A session in the Annex** — the end-to-end proof: a Conversation drafted
   onto *Linux (WSL)* with a mirrored Profile runs under bwrap and commits.
   - The Keeper's Author, mirrored, is on the commit.

## Re-verify at start

- Assumes stage 01 landed: `kept_by`, the Keeper's Join, the forward and the
  linking operation, which takes `data_dir`, `address` and `keeper_at` at
  `POST /api/ui/devices/annexes`, and the direct address each end dials the
  other at first.
- Decide then how the probe reads the Windows host's address on WSL's NAT
  network (the vEthernet (WSL) adapter's, or the Annex's default route
  read from inside), and add the Keeper's peer port to it.
- Assumes the wizard's install plan is `plan()` in
  `crates/server/src/onboarding/install.rs` with a Windows arm run through
  `Elevate::raise`, and the hint screen draws per-row instructions from
  `web/src/setup/instructions.ts`.
- Assumes the NixOS module's options are still `listen`, `peerListen`,
  `paths`, `home` and `shell` in `nix/module.nix`, and that NixOS-WSL's
  current release boots systemd by default and takes the distro's hostname
  from `networking.hostName`.
- Assumes `flake.nix` still pins `nixos-26.05`; the box follows whatever it
  pins then.
- Assumes nixpkgs unstable packages `claude-code`, `codex`, `opencode` and a
  `grok` CLI; drop any it lacks rather than installing a vendor's.
- Assumes the Keeper's startup in `crates/server/src/lib.rs` has a place for
  background work after the listeners are bound, and shutdown a place before
  the sessions are stopped.
- Decide then how a development build names its binary: an environment
  variable naming a Linux binary to copy in, or a flake override.
