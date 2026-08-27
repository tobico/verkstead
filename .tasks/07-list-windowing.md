# 07. List windowing

## What to build

The task-list and stage-list timeline cards stop growing with the backlog. Each
shows a window of **five real items centered on the next undone entry**,
clamped to the list's ends, with a plain ellipsis row above and/or below where
items are hidden. The ellipsis rows do not count against the five. The settled
examples: none of ten done shows items 1–5; five of ten done shows 4–8; nine of
ten done shows 6–10. When every item is done — stage lists outlive their
completion — the last five show.

The window applies to the timeline cards wherever they are drawn (the pinned
card and its copy on the record row alike, which share components). The Backlog
and Roadmap details panes stay complete — they are the place you go to see
everything.

The progress line already on the card keeps counting the whole list.

## Acceptance criteria

- [ ] A ten-item list shows five items windowed per the examples above, with
      ellipsis rows exactly where items are hidden
- [ ] Lists of five or fewer are unchanged, with no ellipsis rows
- [ ] An all-done list shows its last five
- [ ] The details panes still render every entry
