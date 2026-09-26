# 06. COSMIC, and a window a compositor would rather decorate

## What to build

Nothing new. This is the run that puts the finished app on a real COSMIC
session — the desktop stage 03 found it coming up on as a client the compositor
decorates itself, which is the case the overlay has to land in — and writes down
what it does there.

**What is already known, so the run is not spent rediscovering it.** All of this
was probed on the pinned Electron while the stage was planned, with a nested
COSMIC session on this machine:

- The overlay **does** land on COSMIC, both when the app runs over Xwayland and
  when it runs as a native Wayland client. The rectangle is real, it follows a
  resize, and `setTitleBarOverlay` recolours it while the window is open.
- **A real COSMIC session gets the Wayland path.** With `XDG_SESSION_TYPE`
  saying wayland, which a COSMIC session sets, the pinned Electron picks Wayland
  by itself — the app passes no ozone flag and does not need one.
- **And on that path the overlay is one button wide.** Measured in the same
  session, against the same app: 96px and the usual three controls over
  Xwayland, 32px and a close button alone under Wayland. Minimise and maximise
  are simply not in the overlay there.

So what this run is for is the *app* rather than a probe — the heads dragging,
the insets measured against the real rectangle, the recolour on a real theme
flip — and the judgement about that missing pair.

**The one button is the thing to decide about.** A Verkstead on COSMIC that
cannot be minimised from its own window is a worse app than the decorated one it
replaces, and the answers are not this stage's to pick between blind: the
compositor's own gestures may cover it, or it may want a word in the Linux
stage. What this task owes is the finding written where stage 05 reads it, the
way stage 03 wrote its tray findings forward, rather than a fix invented here.

## Acceptance criteria

- [ ] On a COSMIC session: the window draws with no title bar, moves by any pane
      head, keeps its outermost heads clear of the controls, and recolours when
      the scheme flips.
- [ ] What COSMIC does with a double-click on a head, and what it offers in place
      of the minimise and maximise the Wayland overlay does not draw, is written
      down.
- [ ] Whatever is left wanting is written into the brief for stage 05 rather
      than only into this session.
