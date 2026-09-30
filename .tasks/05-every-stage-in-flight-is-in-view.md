# 05. Every stage in flight is in view

## What to build

The stage card stops centring its window on the first stage not done, and shows
**every stage in flight** instead — with the neighbouring stages filling the rest
of the window up to five, and the ellipsis row wherever the roadmap goes on past
either end. The head keeps counting the whole roadmap, as it does now.

Five around the first stage not done was *where the work has got to* while a
roadmap had one such place. With three, none of the stages in flight is dropped to
keep the window at five: what a reader came to the card for is where the effort
is, and three stages running is three rows whatever else has to give way.

*In flight* here is the states the three tasks before this one added — in
progress, waiting to join, waiting for a place — and *halted*, which is a stage
somebody has to do something about and is the last row worth hiding. A stage
*waiting on* another is not in flight.

- **Every stage in flight is a row**, however many there are and however far apart
  in the roadmap they sit.
- **Neighbours fill the window out to five**, so a roadmap with one stage running
  reads as it does today: the stage, what it came out of, what it goes into.
- **The ellipsis rows say what is out of sight** at either end, as they do now — a
  count in words beside the glyph, because an ellipsis read aloud says nothing.
- **A roadmap with nothing in flight reads as it does today**: every stage done
  shows the last five, and a roadmap nobody has started shows its first five.

The window is shared with the backlog's card, which is the same card one level
down and has one place the work is at. Whatever is done here leaves the backlog's
window alone, and a window that read differently on the two would be two ideas of
where the work is.

There is no Playwright here. Where the layout has to be measured rather than
asserted, it is measured by driving the system Firefox — and a `getByRole` that
never matches inside a `waitFor` runs the web suite out of memory rather than
failing, so it is worth writing the query to match before writing the wait.

## Acceptance criteria

- [ ] Three stages in flight are three rows on the card, with the done count in the
      head still counting the whole roadmap.
- [ ] A long roadmap with stages in flight far apart still fits the card without
      pushing the record under the fold, and the ellipsis rows say how many are out
      of sight at each end.
- [ ] A roadmap with one stage in flight, one with every stage done, and one nobody
      has started each draw what they draw today.
- [ ] The backlog's card is unchanged.
