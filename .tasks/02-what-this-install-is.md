# 02. What this install is

## What to build

The server works out at startup what kind of install it is running from, puts
that on the Devices view for **this device**, and this device's row says how to
update by hand where the install cannot update itself. No button yet — task 03
adds it for the kinds that can.

The kinds, read off the server's own executable (the startup already reads it:
`Executable::of_the_server` in `crates/server/src/sandbox.rs`, with the nix
`-wrapped` name unwrapped) and the environment it was started with:

| Kind | How it is told | Updates itself? |
|---|---|---|
| AppImage | started with `--desktop`, `APPDIR` set and the executable inside it (the app keeps `APPDIR` in the sidecar's environment — `desktop/src/unmounted.ts`) | yes |
| Mac app | `--desktop`, executable inside a `.app` bundle's `Contents/Resources/cli` | yes |
| Windows install | `--desktop`, executable under the per-user install directory the NSIS installer uses | yes |
| NixOS / nix | executable under `/nix/store` | no — the line names rebuilding the system |
| Dev checkout | executable under a `target/` build directory, `--desktop` or not (`pnpm start` runs the checkout's build) | no — the line names pulling and rebuilding |
| Anything else | a bare `verkstead serve` | no — the line names downloading the new binary |

- The kind is this device's only: it rides on the Devices view's `this`, not on
  the identity record members read (stage 05 decides what a member tells).
  Add it to the view type and regenerate the viewer's types.
- The row for a kind that cannot update itself shows its version (stage 01's
  line) and the by-hand line, as the brief has it — whether or not *update
  available* is showing beside it. Words follow `docs/copy.md` and use
  `CONTEXT.md`'s vocabulary; a NixOS line reads *update this device with …*.
- Nothing is ever attempted on those kinds.

## Acceptance criteria

- [ ] A NixOS module install with a newer release out reads *update this device with …* on its row, and has no button.
- [ ] A `pnpm start` dev run reads as a dev checkout, not a desktop install; a packed AppImage reads as an AppImage.
- [ ] The detection is unit-tested for every kind on every platform's paths, including the nix wrapper.
