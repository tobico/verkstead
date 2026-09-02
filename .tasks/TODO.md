# Brief composer

The Brief's editing surface moves off the Timeline card and into a details
pane styled as a chat composer: a centered text box with borderless
label-over-value dropdowns along its bottom edge (Repo first, then the three
role pairings) and the start button beneath. The card shows the five-line
rendering at all times. A conversation whose timeline holds exactly one event
hides the timeline entirely, the pane spanning its width too. A new compose
page reuses the same composer against client-held, per-device temporary state,
replacing the New-conversation menu with a button; Start creates and kicks
off, Save as draft creates without starting, and adopting a roadmap becomes a
dropdown that loads the roadmap into the compose state.

Settled in the grilling: frozen briefs of rounds past drafting keep the
existing facts pane — the composer serves conversations while they draft,
including adopting drafts (box locked to the frozen rendering) and later
rounds (branch and base already frozen). Every setup field on a saved
conversation keeps its immediate per-field save; only the compose page holds
unsaved state, and it creates by replaying the existing endpoints.

## Tasks

- [x] 01: The composer pane — [details](01-composer-pane.md)
- [x] 02: The composer look — [details](02-composer-look.md)
- [x] 03: Switching a draft's repo — [details](03-switching-a-drafts-repo.md)
- [x] 04: The one-event layout — [details](04-one-event-layout.md)
- [x] 05: The compose page — [details](05-compose-page.md)
- [x] 06: Pairing prefill on compose — [details](06-pairing-prefill.md)
- [x] 07: Adopting from compose — [details](07-adopting-from-compose.md)
