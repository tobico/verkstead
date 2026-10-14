# 08. The README, and dropping adoption

## What to build

Rewrite the README to the present, and delete `docs/adoption.md` with every
link to it. This is the last task because the README names controls by their
labels, and the tasks before this one renamed them.

The README keeps the essay voice — the documents are not the plain register —
but what it says has to be true of Verkstead as released. Today its status
section says *early, private and unreleased* and *nothing to download yet*,
while v0.1.4 is on GitHub Releases; it leans on `docs/adoption.md` six times
for how to get it running and what a session can reach. Write it to say: what
Verkstead is; that it is released, and where; the four ways to get it — the
NixOS module, the Linux AppImage, the Mac dmg, the Windows msi — with the Mac
and Windows unsigned-app steps (Gatekeeper and System Settings, SmartScreen's
*More info* and *Run anyway*) in a few lines each where adoption had them at
length; the sandbox boundary on each platform in a paragraph; and *Where
things are written down*, with the Copy entry task 01 added and the Adoption
entry gone.

Every control the README names in bold is spelled as the rewritten screen now
spells it, every term is `CONTEXT.md`'s, spelling is British, and nothing in
it links to adoption. Adoption's Languages section — descriptors,
`config.yaml`, store sizes and eviction — goes with the rest: the descriptors
ADR, `CONTEXT.md` and the design doc hold that material, and nothing is moved
out of adoption before it is deleted. Check the rest of the repository for a
link to adoption before it goes; the README is where the links were found.

## Acceptance criteria

- [ ] `docs/adoption.md` is deleted and nothing in the repository links to it.
- [ ] The README's status says what is true of the current release, names the four artifacts and the Mac and Windows first-launch steps briefly, and its *Where things are written down* lists Copy and not Adoption.
- [ ] Every control named in the README matches the label the screen now has, every term is `CONTEXT.md`'s, and spelling is British.
