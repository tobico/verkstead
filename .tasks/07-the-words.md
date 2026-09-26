# 07. The words

## What to build

`CONTEXT.md` gains the term this stage introduces, the two files that promised
this stage stop promising it, and `docs/development.md` says what a dev run now
puts on the screen. This stage amends only what it has actually made untrue —
the platform rewrites are stage 05's onwards, and nothing here retires anything.

**The new term is the window's decorations**: no title bar, the platform's own
controls placed by platform — the traffic lights left of the Wordmark on a Mac,
the controls overlay at the top-right on Windows and Linux — coloured and sized
from the page over the bridge and recoloured when the scheme flips; the pane
heads as the drag region with their controls excepted; the frame keeping its
outermost heads clear by reading the overlay's rectangle; and the bare drag bar
on a page that has no head. With the half that matters most: **none of it
reaches a browser** — a drag region is inert outside the app, every inset is
zero where there is no overlay, and the colours travel over a bridge a phone
does not have. Written in the file's own form, with its own *Avoid* line.

**What has stopped being true**, and nothing else in either place:

- The module docstring on the app's window file closes by saying the decorated
  window is that stage's and the frameless one with the controls overlay is the
  stage after it. It is this stage, and it has landed.
- ADR-0020's window section describes the decorations in the future tense of a
  decision not yet built. The decision is unchanged and the ADR is not being
  rewritten — what it needs is to stop reading as a plan.

**`docs/development.md`** describes what `pnpm start` opens: one window on the
workbench, already logged in. What it gains is that the window now has no title
bar of its own and the controls are the overlay in the head's colours — and, if
task 06 found anything a Linux desktop does differently, the sentence that saves
the next person the hour.

**And the Rust tray app is still what ships.** The trunk stays releasable
through this roadmap — the tray app keeps shipping on each platform until the
stage for that platform takes its release leg, and stage 08 retires it once none
is left. So the packaging, release and toolkit sections are left exactly as they
are.

## Acceptance criteria

- [ ] `CONTEXT.md` carries the term for the window's decorations, in the file's
      own form, and says that none of it reaches a browser.
- [ ] The window file's closing promise and the ADR's future tense are settled,
      and nothing else in either is touched.
- [ ] `docs/development.md` says what a dev run's window now looks like, and the
      Rust tray app is still named as what a release carries.
