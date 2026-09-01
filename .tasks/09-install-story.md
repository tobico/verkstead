# 09. The install story

## What to build

What a reader is told, written once at the end when what is true has settled.

**The dmg joins the install story stage 03 started.** `README.md` and
`docs/adoption.md` both tell it as the flake and the AppImage today; the macOS
app is the third way in, told beside them rather than merged into them — the
flake stays daemon-only, and the three are told apart so a reader can tell which
of them they want.

**Gatekeeper.** The app is unsigned, so a Mac reader who double-clicks it is
refused with no obvious way past. The steps through System Settings that get
them running go beside the download rather than in a troubleshooting section, on
the grounds that everybody who downloads it needs them.

**Sessions on a Mac.** The README says Verkstead is Linux-only because bwrap is
a hard requirement, and after this stage that is no longer true. What replaces it
says what a Mac session actually is: the same Sandbox by intent, over Apple's own
mechanism, where the boundary refuses rather than hides — and what stays the
machine's, as the AppImage's section already lists what stays the machine's
there. Windows is still without sessions, and stage 05 is where that is said.

**`docs/releasing.md`** names what a tag now produces, with no count the workflow
contradicts.

## Acceptance criteria

- [ ] A Mac reader can find the dmg, get past Gatekeeper with what is written,
      and knows what a session on their machine can and cannot reach
- [ ] Nothing left in the docs says the product or the desktop app is Linux-only
- [ ] `docs/releasing.md` matches what the workflow now builds and attaches
