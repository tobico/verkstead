# 01. The machine on the wire, and the rail

## What to build

The wizard's frame as the wide boards draw it (`01-choosing` and the rest
under `docs/design/setup-wizard/`), with every step's contents left as they
are inside it. Two things, one of them on the wire.

**The reading names the machine.** The onboarding view gains a `machine`
field: this server's hostname, the same word the devices pane and the install
run's status line already use for it. The render crate declares it, the server
fills it from the hostname helper the platform module already has, the golden
fixtures the server's onboarding test writes into the web suite's fixtures
folder carry it, and the generated viewer types follow. This is the one wire
change in the whole backlog, chosen over *Reading this machine every 10 s* and
over reading the devices endpoint for the name alone.

**Rail and paper from 60rem**, the breakpoint the workbench already breaks at
(`BESIDE` in the viewer's widths module), so the wizard and the workbench
change shape at the same width. Below it the page stays the one column it is
today; task 02 makes the phone layout. The rail is 290px on the hunk colour
(reuse `--hunk` with a comment saying the rail is drawn on it; no new token),
and holds:

- The brand mark and the word **Verkstead** as the page's visible `h1`, with a
  SETUP label under it. The h1 *Set Up Verkstead* goes; the app test that finds
  the wizard by it finds the brand instead.
- The three steps as rail items, each an `h2` carrying `data-step`,
  `data-met` and `aria-current` as the frame does today, so the frame tests
  keep finding them. An item is a numbered ring while unmet, a tick while met,
  and the open one sits on a card. **It is a button only when met and not
  open**, today's rule: an unmet step is plain text because pressing it would
  be the skip ADR-0016 refused.
- Under a met step, a one-line summary computed in the viewer. Step 1 names the
  present rows in their row names with the serial comma copy.md asks for: *A
  sandbox, git, and Claude Code*. Step 2 names the harnesses of the profiles
  saved, *Claude Code* or *Claude Code and Codex*, read from the profiles list
  (its own endpoint, which the accounts step already writes to and
  invalidates) rather than counted off the reading. A step 3 that is met is the
  wizard over, so it has none.
- At the foot, a green dot and the reading line: *Reading ada-box every 10 s*,
  and *Reading ada-box every 2 s while it installs* while a run is going, the
  two numbers being the frame's `PROBE` and `RUNNING` cadences.

The paper to the right carries an ember eyebrow *Step N of 3*, the step's
title in Title Case as `TITLES` has it today (copy.md's heading rule stands
over the boards' sentence case; this was asked and settled), the standing line
under it, the step's own contents, and **a foot the frame owns**, pinned to the
bottom with Back on the left and the step's primary press on the right. Back
opens the step before, which is always met. Each step says what goes in the
foot rather than drawing its own press under its rows; in this task the press
keeps reading **Next** and behaves exactly as today, the labels being the
later tasks' to change. The drag bar stays above everything in the desktop
app.

Keep the `h2` levels and the data attributes so `web/tests/onboarding.test.tsx`
changes only where the words did. Type sizes map from the boards: title
1.625rem, body 1rem, secondary 0.85rem, labels 11.5px uppercase with 0.05em
tracking.

## Acceptance criteria

- [ ] The onboarding reading carries `machine`, the fixtures and generated
      types carry it, and the rail reads *Reading <machine> every 10 s*, and
      *every 2 s while it installs* during a run.
- [ ] From 60rem the page is a 290px rail on the hunk colour beside the paper;
      the rail's brand is the visible h1, each step is an h2 rail item that is
      a button only when met and not open, and a met step shows its summary.
- [ ] Each step's press stands in the frame's foot beside Back and does what it
      did; the frame, dependencies, accounts and git suites pass against the
      new markup.
