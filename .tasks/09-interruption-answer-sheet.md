# 09. Interruption as an answer sheet

## What to build

An Interruption stops being the one event answered where it stands in the
timeline. Its card becomes a plain openable button like a question set's:
the whole surface opens the details pane, the "blocked on you" badge stays
on it while unsettled, and once settled the card names the remedy chosen.
The separate Evidence button goes — opening the card is opening the
evidence.

The details pane renders the interruption **like an answer sheet**. The
evidence — what the worktree looked like, what the session last said — sits
above the remedies, read the way a set's preface and diff are read before
answering. Below it the three remedies (retry, take over manually, abort the
run) become option rows in the answering idiom: pick one, write the optional
note beneath, and press one submit. Nothing acts on a stray tap — settled
explicitly against relocated one-tap buttons, both for safety around abort
and because it is exactly how answering already works.

Each remedy keeps its explanatory note, and the reassurance that the
worktree is left as it is stays with the sheet. A settled interruption's
pane shows the record as an answered sheet would: the evidence, the remedy
chosen, the note that went with it.

## Acceptance criteria

- [ ] The timeline card is a plain button whose whole surface opens the
      pane; badge while unsettled, chosen remedy named once settled
- [ ] In the pane the evidence renders above the remedies, and a remedy
      takes effect only when picked and submitted, never on a single tap
- [ ] A settled interruption reads in the pane as an answered sheet:
      evidence, chosen remedy, note
