# WSL Annex roadmap

A Verkstead on a Windows machine makes a second Verkstead inside WSL — a
NixOS it writes the configuration of, boots with itself, links into the
cluster with no press from anybody, and keeps up to date — so that a tick in
the setup wizard is what Linux development on Windows costs. The second
Verkstead is an **Annex** and the first its **Keeper**, and the shape is
generic enough that a Docker box on a Mac or a Windows VM on a Linux desktop
is the same thing with another backend. The decisions and their why are in
[ADR-0024](../../adr/0024-the-annex.md); [CONTEXT.md](../../../CONTEXT.md)
carries the terms.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

Stages 01 and 02 are platform-neutral and stand on nothing; either may land
first. 03 is the Windows work and stands on 01, whose Keeper's Join and
forward are what it links the box with. 04 stands on 03, there being nothing
to keep until something is made. 02 is wanted by the time an Annex is used in
anger — a box with no repository in it is a box nothing can be drafted onto —
but nothing in 03 or 04 calls into it, so it is not declared under either.

## Stages

- [x] 01: The Annex and its Keeper — [brief](01-the-annex-and-its-keeper.md) — no dependencies
- [x] 02: Clonable repos — [brief](02-clonable-repos.md) — no dependencies
- [ ] 03: Making the Annex — [brief](03-making-the-annex.md) — after 01 — on windows
- [ ] 04: Keeping the Annex — [brief](04-keeping-the-annex.md) — after 03 — on windows
