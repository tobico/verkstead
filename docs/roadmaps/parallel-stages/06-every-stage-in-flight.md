# 06. Every stage in flight

## Goal

The pinned stage card and the roadmap's details pane show **where every stage
of the roadmap is**: *waiting on* a named stage, *in progress*, *waiting to
join*, or *done* — each stage in flight leading to its own Conversation. With
three stages running, all three are in view.

## Decisions in force

- **[ADR-0021](../../adr/0021-parallel-stages.md), *The viewer***.
- **The card stops centring on one stage.** `windowing.ts` shows five entries
  around the first one not done, which was *where the work has got to* when
  there was one such place. With several, the stages in flight are what is
  worth seeing, and none of them is hidden to keep the window at five.
- **The states come from the server's record**, not from the boxes and not
  from the annotation: the viewer is told a state, it does not work one out.
- **Waiting on names the stage waited on** — and where there are several, the
  ones not yet settled.
- **A stage's platform is shown where it has one**, as stage 01 shows it on
  the details pane.
- **An undeclared roadmap reads as it does today**, with the one stage being
  worked marked *in progress* — which it never was before, there being no such
  state to show.

## Proposed tasks (provisional)

1. **A stage has a state** — the stage list the viewer is sent carries each
   stage's state and, where it is in flight, its Conversation. AC: the four
   states are told apart; a halted stage is told apart from one in progress.
2. **The card** — every stage in flight is in view, with the done count beside
   them. AC: three in flight are three rows; a long roadmap still fits the
   card; a row leads to that stage's Conversation.
3. **The details pane** — the contents line and the section carry the state in
   place of *done* and *to do*. AC: it moves while open, as a tick does today.
4. **Waiting for a place**, if stage 05 has landed. AC: told apart from
   waiting on a stage.

## Re-verify at start

- Stage 04 landed. Stage 05 is reorderable with this one: where it has
  landed, its waiting is a state to show; where it has not, leave room for it.
- `StageEntry` and `StageDocument` in `crates/render` are still what the card
  and the pane are fed, and carry `done` alone.
- The card is a pinned Event on every stage's Timeline and on the roadmap's
  own. Check it says the same thing on each.
- There is no Playwright here; the layout is measured by driving the system
  Firefox, and a `getByRole` that never matches inside a `waitFor` runs the
  web suite out of memory rather than failing.
