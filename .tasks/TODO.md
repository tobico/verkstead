# Memo card title

A memo is named by the session that leaves it, and the Timeline shows the name
rather than the number. `verkstead memo <title>` takes the title as its one
argument with the markdown still on stdin; the card is headed *Memo* with the
title as its subtitle and nothing else; the pane is headed by the title, with
the whole memo under it and an understated line at the bottom saying who left
it and when — the session a link to its own output — with Retire or Restore
beside that line. The id is drawn nowhere on screen and stays the session's
handle for retiring, which the prompt still names.

And the selection no longer walks past a waiting Question Set: a memo landing
after a Set used to open the memo and leave the Set unanswered behind it, so
the landing now stops at the oldest Set still waiting until it is answered.
The decisions are in ADR-0023's amendment *a memo has a title, and the card
shows it*, and the terms are **Memo** and **Landing** in CONTEXT.md.

## Tasks

- [ ] 01: A memo has a title — [details](01-a-memo-has-a-title.md)
- [ ] 02: The card and the pane — [details](02-the-card-and-the-pane.md)
- [ ] 03: The selection stops at a waiting Set — [details](03-the-selection-stops-at-a-waiting-set.md)
