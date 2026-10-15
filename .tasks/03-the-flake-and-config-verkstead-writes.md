# 03. The flake and config Verkstead writes

## What to build

The Keeper writes the Annex's whole configuration (ADR-0024, *The box on
Windows*) as files it will put in the distro in task 04. This task is the
writer and its proof. It does not touch WSL, so it is proven with nix on
Linux.

**The flake**, `/etc/nixos/flake.nix` with its lock produced by the first
rebuild. Its inputs:

- `nixpkgs` on the channel the repository's own `flake.nix` pins
  (`nixos-26.05` today). Read the channel from the repository, not from a
  hard-coded constant, so the two cannot drift.
- `nixpkgs-unstable`, used for the harnesses alone.
- `nixos-wsl` at the release whose image task 04 downloads (`2605.7.2`
  today, matching 26.05). Image and module move together.
- `verkstead` at the Keeper's own release tag (`v<CARGO_PKG_VERSION>`), for
  `nixosModules.default` only.
- **The release binary of that version as a locked non-flake input**:
  `verkstead-linux-x64` from the GitHub release at that tag, a static musl
  binary. It never comes from `nix/release.json`, because a tag's own tree
  names the previous release there.

**The configuration**:

- The module with `listen = "127.0.0.1:8426"` and
  `peerListen = "0.0.0.0:8425"`. These ports are the Annex's own and never
  8422/8423, because under mirrored networking it shares the Keeper's
  loopback.
- `package` set to the binary input, wrapped as the release package is (git
  on `PATH`).
- `paths = [ "/srv/repos" ]`, with the directory created and owned by the
  service user.
- `advertising = false`. An Annex is never advertised over mDNS. The server
  already skips it when kept, so this is belt and braces.
- `wsl.enable` and NixOS-WSL's defaults (systemd is native).
- `networking.hostName` set to the Keeper's hostname, which NixOS-WSL takes
  for the distro.
- Packages: bubblewrap, git, gh, nix, sccache, and `claude-code`, `codex`
  and `opencode` from the unstable input. **No grok**: nixpkgs' `grok-cli` is
  superagent-ai's tool, not xAI's Grok Build. This was decided, and the ADR's
  list changes with it.

**`config.yaml`**, for the Annex's data directory: `kept_by` with the
Keeper's id and fingerprint (task 05's link rewrites it with `at`, using the
existing `Settings::keep_by`), and the Keeper's `git_author`.
**`secrets.yaml`**: the Keeper's `github_token`. Nothing else is copied;
stage 04 copies the rest. Write both through the settings types, so the
Annex reads them exactly as it writes them.

**A development build's binary** (decided): when `VERKSTEAD_ANNEX_BINARY`
names a Linux binary, that file is what goes in. Task 04 copies it into the
distro, and the configuration uses it as `package` in place of the release
input. The release tag still supplies the module. A development build
reports the same `CARGO_PKG_VERSION` as the release, and v0.1.5 predates
`kept_by`, so this override is how every end-to-end proof in this stage
runs until a release carries stages 01–03. Document the variable in
`docs/development.md`.

Update ADR-0024's harness list (no grok) and record the override there.

## Acceptance criteria

- [ ] The written flake pins the `verkstead` input and the binary input to
      the Keeper's own version, and nixpkgs to the repository's channel.
- [ ] In the Linux suite, `nix flake check` or an eval of
      `nixosConfigurations.<name>.config.system.build.toplevel.drvPath`
      succeeds on the written flake. It may skip where nix is absent, but it
      runs in CI's nix job. The module's `listen`/`peerListen`/`paths`, the
      hostname and the package list read back from the evaluated
      configuration.
- [ ] With `VERKSTEAD_ANNEX_BINARY` set, the evaluated `package` is the
      copied-in binary and no binary input is fetched.
- [ ] The written `config.yaml` and `secrets.yaml` read back through the
      server's own settings types: `kept_by`, the git author and the token.
