# 04. Client-side decorations

## Goal

The window has no title bar. On Linux the platform's controls overlay sits at
the top-right corner in the heads' own colours, light and dark; every pane
head is the drag region with its buttons excepted; the frame keeps the
outermost heads clear of the controls, in three panes and in one; and a page
with no pane head — onboarding, the no-such-page — has a bare drag bar of the
same height, inside the app only. The traffic-light inset for a Mac is
written here and proven in stage 06; the Windows overlay is the same code as
Linux's and proven in 07.

## Decisions in force

- **Native controls, placed by platform** ([ADR-0020], Set 847 Q11, the
  comment on Set 846): traffic lights left of the Wordmark on a Mac, the
  controls overlay at the top-right on Windows and Linux. Custom controls
  drawn by the page were rejected for losing snap layouts and the platform's
  own drawing.
- **Pane heads are the drag region**, their clickable controls excepted. The
  frame — not each pane — pads the leftmost and rightmost heads clear of the
  controls, reading the overlay's rectangle; a narrow window's single pane is
  both.
- **Headless pages get a bare drag bar** (Q11a), the head's height, drawn
  only where the bridge exists.
- **Colours come from the page** (Q11b): the page pushes its head colours
  over the bridge whenever the theme flips, and the main process recolours the
  overlay. Two fixed colours were rejected.
- **The menu bar stays hidden** on Windows and Linux (stage 02); a Mac keeps
  its application menu.
- **Nothing about this reaches a browser**: a drag region is inert outside
  the app, and every inset is zero where there is no overlay.

## Proposed tasks (provisional)

1. **Frameless, with the overlay** — the window created with the title bar
   hidden and the overlay on, sized to the head's first row. Accepts: the
   window draws with no title bar; the controls work; double-click on a head
   maximises.
2. **Drag regions** — the pane head component is draggable, every interactive
   child in it is not, and the settings, compose, share and set pages' heads
   inherit it. Accepts: the window moves by any head; every button in a head
   still presses.
3. **Insets from the frame** — the overlay's rectangle read on the page,
   handed to the frame as variables, the outermost heads padded; the
   Wordmark's left inset for a Mac written now. Accepts: no control in the
   details head sits under the overlay in three panes, two, or one; the
   sidebar's head leaves room on the left when the platform says Mac.
4. **Colours over the bridge** — head colours pushed on load and on theme
   change. Accepts: a theme flip recolours the overlay without a restart.
5. **The bare drag bar** — onboarding and the no-such-page draw one in the
   app. Accepts: the setup page moves by its top; a browser shows nothing.
6. **Proof** — the frame's inset arithmetic and the drag classes under the
   viewer's suite with a stub bridge. Accepts: the suite covers a head with
   and without a bridge, and a page without a head.

## Re-verify at start

- Stage 03 landed: the bridge exists to say the platform and carry colours.
- The pane head is still one component (`PaneHead.tsx`) that every pane's
  head goes through, and the Wordmark still heads the sidebar and the
  list-less compose page.
- Which pages exist with no pane head at that time.
- Whether Electron's overlay on the pinned major supports Linux as it does
  Windows, and what `navigator.windowControlsOverlay` reports there.
- The head's first-row height in both themes, which the overlay's height has
  to match.

[ADR-0020]: ../../adr/0020-electron-desktop.md
