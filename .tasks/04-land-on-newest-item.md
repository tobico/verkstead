# 04. Conversation cards land on the newest item

## What to build

Opening a Conversation from the sidebar takes the human to the end of its
record: the Timeline scrolled to the bottom, with the last item selected and
open in the details pane.

The sidebar's list does not hold timelines, so a card cannot link straight to
the last event's id. Instead (settled in the grilling, over sending the id in
the list payload or a `/latest` path): the card navigates to the
Conversation as today, and once the Timeline has loaded, the page selects the
**last openable item** — the last Timeline event with a full self to open,
skipping the kinds with no pane behind them (a move, a steer that carried no
document) — and rewrites the URL to its path with replace. A Timeline whose
events none can open selects nothing, and the pane stays bare.

This lands only when a Conversation is entered without a detail already in
the URL; a cold load of a detail URL keeps its own selection (task 03).

On a phone the card still lands on the Timeline level — the newest item is
marked open and the details are one tap away — never straight into the
details pane.

And the Timeline follows its bottom the way a session's output already does:
opening a Conversation scrolls to the bottom, and while a session is running
the view keeps following new events chat-style, until the human scrolls away
— and again once they return to the bottom. The follow-bottom behaviour the
output pane uses is the pattern (and likely the code) to share.

## Acceptance criteria

- [x] Pressing a conversation card opens the Timeline at its bottom with the
      last openable event selected, its path in the URL via replace
- [x] Events with nothing to open are skipped when picking the last item; an
      unopenable record selects nothing
- [x] While a session runs, arriving events keep the Timeline pinned to the
      bottom unless the human has scrolled up; a phone lands on the Timeline
      pane, not the details pane
