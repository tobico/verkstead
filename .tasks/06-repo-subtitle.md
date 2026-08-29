# 06. The repo subtitle in the pane head

## What to build

The Timeline pane's header shows which Repo the Conversation is in, understated
beside the branch title — the same title/subtitle pattern the sidebar's
conversation card draws its name and repo in, so the header and the card read
as the one name said twice. Drawn in every state, including on a Draft, where
the title reads "Draft" and the repo is the one thing that tells two drafts
apart.

The Timeline pane head only — no other pane gains it. It has to stay legible
at phone widths, where the header row also carries the way back and the way on
to the details pane.

## Acceptance criteria

- [ ] The Timeline pane head shows the repo understated beside the title, in every state including Draft
- [ ] The header row still lays out cleanly at narrow widths
- [ ] A web test covers the subtitle's presence and wording
