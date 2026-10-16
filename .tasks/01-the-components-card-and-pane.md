# 01. The Components card and pane

## What to build

A **Components** card on the settings page, and the pane behind it, holding
one row: *Linux development (WSL)*, the Annex (ADR-0024; CONTEXT.md,
*Components*). The name is *Components* rather than *Annex* because the human
named the card as the place where installing more later goes, such as a harness
added after setup. The Annex's row is its first row and does not define it.

**The row reads the machine's state, and the server answers it.** Add a reading
to the server that the pane draws from:

- **The state**, from the Annex backend's probe: *not installed* (no WSL, or WSL
  with no distro made to the end), *running*, *stopped*, and *updating* (which
  tasks 03 and 04 set; this task only needs the state to exist and be drawn).
  Where the probe says a restart is needed or virtualization is off, the row
  says so in the wizard row's own words.
- **The version**: the Verkstead release the box is pinned to. This is
  **the version the Keeper last pinned, held since its start**, and the box is
  never started just to read it (Set 30 Q7). Read it off the box's
  `/etc/nixos/flake.nix` (the `verkstead` input's tag) when the Keeper boots the
  Annex, hold it beside the boot handle, and draw that, including on a stopped
  Annex. Before the first read, draw no version rather than a guess.
- **The disk**: the size of the box's disk image file under the backend's disk
  directory (`%LOCALAPPDATA%\Verkstead\wsl`). Reading a file's size on the
  Keeper's own disk is enough, and it needs nothing from inside the box.
- And the row's trouble line, where the boot left one (the same line the
  Devices row carries).

**Where the card is drawn:** only on a machine that can have an Annex, which
today is a Windows Keeper, until Components has another row (Set 30 Q4).
Anywhere else there is no card and no pane, and the middle pane's order is
otherwise unchanged. The Desktop card is the model for a card drawn only where
it means something.

**Install and Update are drawn but are this stage's later tasks.** Draw
**Install** on a *not installed* row and **Update** on an installed one only
once those tasks wire them. This task may leave both out entirely, or draw them
unwired; do not ship a button that does nothing silently.

**The Devices row's press.** On the Keeper, the Annex's row on the Devices list
(Remote.tsx; the row wearing the Annex mark, *annex of this device*) gets a
press that opens the Components pane, through the settings page's own path
arithmetic, the way any card opens its pane. **The Annex's row on any other
device has no press** (Set 30 Q5): Components belongs to the Keeper. The row
still has no Unlink.

Read `docs/copy.md` before writing a word of the card, the pane or the states.
Use the CONTEXT.md vocabulary: *Annex*, *Keeper*, *Components*.

## Acceptance criteria

- [ ] On a Windows Keeper with no Annex, the Components pane's row reads *not
      installed*. A machine that cannot have an Annex draws no Components card.
- [ ] With a running Annex, the row reads *running*, the Keeper's version, and
      the disk image's size. A stopped Annex reads *stopped* with the version
      last pinned, and the box is not started to draw it.
- [ ] On the Keeper, pressing the Annex's Devices row opens the Components
      pane. On another device the same row has no press.
- [ ] The reading is covered by a server test over the stub `wsl.exe` that the
      annex suite already uses, and the pane by a viewer test.
