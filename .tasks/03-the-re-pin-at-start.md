# 03. The re-pin at start

## What to build

**The Annex runs its Keeper's release** (ADR-0024). At the Keeper's start,
before the boot that links the Annex, the Keeper compares the version the box's
flake pins with its own. Where they differ, it **re-pins and rebuilds before
booting the Annex into use**, so the rebuild never lands on live work. The
Components row reads *updating* meanwhile, and the Annex's Devices row stays as
it is (*unreachable* until it answers).

The re-pin is:

1. **The flake and configuration written whole again** from the new release's
   templates, exactly as the install run writes them. A new release may change
   the templates (a new package, a NixOS-WSL pin, a workaround dropped), and
   both files are Verkstead's own, with nothing to merge.
2. **Only the Verkstead inputs moved in the lock**: the `verkstead` flake input
   and the `verkstead-binary` input. `nixpkgs`, `nixpkgs-unstable` and
   `nixos-wsl` keep what `flake.lock` holds. Those move on Update (task 04),
   and NixOS-WSL moves when a release bumps its pin (Set 30 Q2; ADR-0024 as
   amended on this branch). A changed `nixos-wsl` URL in the new template
   relocks that input by itself.
3. **`nixos-rebuild switch`**, with its last lines kept as the row's trouble
   line on failure, the way the install run's rebuild failure is reported.
4. Then the boot and link that the start always runs.

**A development build** (`VERKSTEAD_ANNEX_BINARY` set) reports the last
release's version and is not that release, so comparing versions says nothing.
For one, the re-pin runs whenever **the copied binary's bytes differ** from the
one in `/etc/nixos`: copy it in again, write the files, and rebuild.

The comparison, the write and the rebuild all go through the Annex `Backend`
(`run`, `follow`, `write`, `read`), so the whole thing is provable over the stub
`wsl.exe` the annex suite already uses. The *updating* state is held on the
same boot handle that task 01's reading draws from, and the version held there
is the newly pinned one once the re-pin lands.

A re-pin that fails leaves the Annex on its old release. The row then carries
the trouble line, the Keeper carries on, and the next start tries again.

## Acceptance criteria

- [ ] A Keeper one release ahead of its Annex rewrites the flake, relocks only
      the Verkstead inputs, rebuilds, and then boots and links an Annex
      reporting the Keeper's own version, with nixpkgs' locked revision
      unchanged. Proven over the stub, and once on Windows.
- [ ] A Keeper of the same release boots without rebuilding.
- [ ] With `VERKSTEAD_ANNEX_BINARY` naming a changed binary, the start copies
      it in and rebuilds. With an unchanged one, it does not.
- [ ] The Components row reads *updating* during the re-pin, and a failed
      rebuild leaves its trouble line on the row and the Annex on its old
      release.
