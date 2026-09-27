# 03. The overlay, and the window on Windows

## What to build

Stage 04's decorations proven on Windows, in the msi's window rather than in a dev
shell's — which is the whole of what this task is expected to be. The code has no
platform branch in it: `titleBarStyle: "hidden"` is the whole of what asks for the
overlay, the paper, the ink, the band and the row's middle cross the preload bridge
from the page on load and again at every flip of the machine's colour scheme, and
the frame reads what the controls left it and pads the header at each edge by that
much. None of that was written for one platform, and none of it has ever been run
on this one.

**So what this task is, is the looking** — and then whatever the looking turns up.
ADR-0020 took the platform's own controls over controls the page draws precisely
so that Windows' snap layouts keep working, and this is the stage that discharges
that. The same shape as stage 05, which measured what the overlay actually held on
a Wayland session and wrote down what it found.

What to put in front of a real Windows machine:

- **The overlay in the heads' colours.** It stands at the top-right, drawn on the
  paper a pane head is drawn on with its symbols in the head's ink, and as tall as
  the band the page pushed. A machine in the dark scheme gets the dark paper, and a
  flip while the app is running recolours it in that run rather than at the next
  launch.
- **Nothing underneath it.** The header at each edge of the window is padded by
  what `getTitlebarAreaRect()` says the controls took, read again whenever the
  window changes shape — so no control of the page's own sits under the three
  buttons at any pane count or window width, including the narrow window whose one
  pane is at both edges at once.
- **Snap layouts.** Hovering the maximise button is what Windows offers them from,
  and it is the reason the platform's controls were kept.
- **The gestures either side of the overlay**: a double-click on any pane head
  maximises the window and a second one restores it, and `Ctrl+M` minimises from
  the hidden menu bar under **Window**.
- **The taskbar.** The Start-menu entry the msi writes carries
  `System.AppUserModel.ID` naming the app id, which is how Windows ties a running
  window to the entry that started it; a window Windows cannot tie to one draws as
  a second, unnamed icon beside it. Whether Electron reports the same id for the
  window — the app id is in `electron-builder.yml` and the `syncDesktopName`
  arrangement on Linux is the same thought — is what to check, and setting it
  explicitly is the fix if it does not.
- **And the band on a machine that is not at 100%.** The height the page pushes is
  the head's own rules added up against the rem its browser is drawing, so display
  scaling and a larger text size are already meant to be answered; a Windows
  machine at 125% or 150% is the first one to have asked.

**What is not this task.** The close policy and the Desktop page's Windows shape
were written in stage 03 and are the same code the Linux AppImage already proves;
they are worth a glance while there is a window open, but a finding there is a bug
to raise rather than scope to take on. Launch on Startup is task 04's.

**And if the looking finds nothing, that is the task delivered.** Say so where
somebody will read it — the roadmap took a risk on this and the record of it
holding is worth as much as a fix would have been.

## Acceptance criteria

- [ ] No control of the page's own sits under the overlay at any pane count or
      window width, and the header at each window edge is padded by what the
      controls left, after a resize and a maximise as well as at load.
- [ ] Hovering the maximise button offers Windows' snap layouts; a double-click on
      a pane head maximises and restores; `Ctrl+M` minimises.
- [ ] The overlay is drawn on the head's paper in the head's ink at the head's
      height, and follows a light/dark flip in the run it is pushed in — checked at
      100% display scaling and at one above it.
- [ ] The taskbar groups the window with the Start-menu entry the msi wrote rather
      than drawing a second, unnamed icon beside it.
