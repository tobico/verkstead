# 02. Every pane head drags the window

## What to build

The pane head becomes the drag region, and every control standing in it is
excepted. The window moves by whatever head is at the top of the page, and
every button, menu trigger, switch and link in that head still presses.

**One component, so one change.** `PaneHead` really is the single head every
pane goes through — twenty-five callers as this stage starts, the settings
panes, the composer, the Set sheet, the unreadable-Set notice, the repos and
profiles lists and the share's details among them — and the Wordmark is that
same head handed a class for its heading. So the settings, compose, share and
Set pages inherit the drag region rather than each being given one, and nothing
about this task is per-pane.

**Excepted by a rule over the head, not by a list.** A head carries buttons, the
⋯ menu's trigger, switches, icon buttons and the wrapping "← Timeline" way out,
and a codebase that marked each one would be a codebase where the next control
added to a head is the one nobody marked. So the head's own stylesheet says that
what is pressable inside it does not drag — written as a selector over the
head's descendants, so it holds for controls added later. The way back out is
the one to look at twice: it is a button that takes the whole width of the row,
so a rule that missed it would leave a head that could not be dragged where the
title wrapped.

**Text selection in a head goes, and that is the decision.** A drag region
cannot also be text somebody sweeps a pointer across, so a Conversation's name
and the Timeline's truncated branch name stop being selectable. ADR-0020 settled
the heads as the drag region knowing that; it is written here so the next reader
does not take it for a bug.

**And none of it reaches a browser.** `-webkit-app-region` is inert outside the
app, so the same document served to a phone behaves exactly as it does today —
which is also why the standalone share keeps the class it inherits from this
head rather than being given a build of its own without it (settled while
planning).

## Acceptance criteria

- [ ] The window moves by dragging any pane head — the sidebar's, the
      timeline's, the details pane's, the settings' and the composer's.
- [ ] Every control in a head still presses: the gear, the ⋯ menus, the record
      switch, the status buttons and the "← Timeline" way out.
- [ ] The viewer's suite covers the head carrying the drag region and its
      controls carrying the exception, and the same page in a browser being
      unchanged.
