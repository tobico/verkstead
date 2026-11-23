# 01. The box logs in as verkstead

## What to build

The Annex's box makes `verkstead` its WSL login user. `wsl.defaultUser`
becomes `verkstead`; the user gains a real home at `/home/verkstead` and
wheel, so the shell `wsl -d Verkstead` opens is the service's own user with
sudo, and a repository cloned by hand there is owned by the user the server
runs git as — no dubious ownership, no ACLs, no `safe.directory`. The home
joins `/srv/repos` in the box's `paths`, so the service's namespace holds
both.

The NixOS module itself is untouched: the user stays the module's system user
with the service's `HOME` under the state directory, and the box's own written
configuration is what gives the user its home and groups and names it to WSL.
Nothing migrates at the flip — `/home/nixos` stays on disk, nothing logs in
there any more, and the human runs `gh auth login` once in the new shell.

## Acceptance criteria

- [ ] The written configuration names `verkstead` as the WSL default user,
      gives it a real home and wheel, and binds the home into the service's
      `paths` beside `/srv/repos` — and the flake tests in `annex/flake.rs`
      cover each.
- [ ] The module under `nix/` is unchanged, and the service's own `HOME`
      stays under the state directory.
- [ ] Nothing copies, removes or migrates `/home/nixos`.
