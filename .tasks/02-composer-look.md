# 02. The composer look

## What to build

Restyle the composer pane as the chat composer common to AI web apps: one
large text box centered in the details area at the app's measure, with the
configurable options drawn along the inside of its bottom edge as a row of
borderless dropdowns that read as part of the box. The start button is the
only thing outside the box — beneath it, aligned right.

Every dropdown is a dimmed label line over a value. The Repo dropdown comes
first: label **Repo**, value the repo's name, with `+1`, `+2`… appended per
companion. It opens one flat popover panel — not a nested menu — stacking the
repo-shaped setup the way the setup card stacks it today: branch name, base
branch, the add-companion control and the companion rows with each one's
access, base and branch. (The repo picker itself joins this panel in
task 03.) The three role dropdowns follow, the role as label — **Grilling**,
**Implementation**, **Review** — and the chosen Pairing as value, read the
one way a Pairing reads everywhere.

Fields already frozen for the round simply are not offered, as the setup card
does today; what a control cannot do it does not draw.

## Acceptance criteria

- [ ] The box is centered with the option row inside its bottom edge —
      borderless triggers, dimmed label over value — and the start button
      beneath it on the right; light and dark themes both hold up.
- [ ] The Repo trigger reads the repo name plus `+N` for companions and opens
      a single popover panel holding branch name, base branch and the
      companion rows, each control keeping its immediate save and refusals.
- [ ] The three role triggers read role over Pairing and open the same picks
      the setup card offers, No-grilling and No-review rows included.
- [ ] CONTEXT.md's Brief entry describes the composer pane instead of the
      card-is-the-field setup card.
