# 07. The record

## What to build

The documentation follows the one-binary story. `docs/adoption.md` describes
the artifacts as they now are — the AppImage and dmg wording, the msi in place
of the portable exe, SmartScreen's clicks retold for an msi — and nothing
anywhere tells a reader to run `verkstead-desktop`. `CONTEXT.md` is swept for
the same. `nix/module.nix` stays untouched, by decision on the grilling.
ADR-0012's amendment already landed with the plan commit; this task is the
rest of the prose catching up.

## Acceptance criteria

- [ ] No document tells a reader to run `verkstead-desktop`.
- [ ] `docs/adoption.md` describes the msi and its SmartScreen clicks.
- [ ] A sweep for the old binary's name outside the crates and the ADRs' own
      record comes back empty.
