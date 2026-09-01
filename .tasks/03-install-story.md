# 03. The install story and what a tag now produces

## What to build

The documentation catches up with the workflow, in the two places that now
disagree with it.

**`docs/releasing.md`** describes a tag as publishing "the four". It names what
a tag produces now — the CLI binaries and the Linux desktop artifact — with no
count left in it that the workflow contradicts, and the after-the-run checklist
covers the AppImage the way it covers a downloaded binary.

**The install story gains its Linux desktop half.** `README.md` and
`docs/adoption.md` both tell getting Verkstead onto a machine as the flake and
nothing else. The AppImage is now the other way in, and the two are different
things rather than alternatives: the flake and the NixOS module run the headless
daemon, and the AppImage is the desktop app a person starts from an icon. Tell
them apart so a Linux reader can see at a glance which of the two they want, and
write it in the same unreleased framing the rest of the documentation uses —
what a tag produces, not a download url pretending a release has happened.

Where the AppImage is introduced, say plainly that a desktop with no tray host —
vanilla GNOME without the AppIndicator extension is the case people meet —
shows no icon, and that Verkstead serves and opens the browser regardless. It is
the one thing about this artifact that looks like a failure and is not.

## Acceptance criteria

- [ ] No count or list in `docs/releasing.md` that a tag's actual output
      contradicts.
- [ ] A Linux reader of `README.md` or `docs/adoption.md` can find the AppImage
      and can tell whether they want it or the flake, without either being
      described as a replacement for the other.
- [ ] The tray-host caveat is written where the AppImage is introduced, and says
      what still works.
