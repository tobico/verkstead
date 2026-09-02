# 07. Adopting from compose

## What to build

Adopting a roadmap moves into the compose page, and the old New-conversation
menu retires — the button from task 05 stands alone.

An **Adopt a roadmap** dropdown sits underneath the text box, anchored left —
to the left of the buttons. It reads as an action with a chevron rather than
a label-over-value option, and it is drawn only when there is something to
adopt *and* no brief text has been typed. Its rows are the abandoned
roadmaps as the menu words them today: the roadmap's name, its repo, the next
stage and, where not the default, its base.

Picking one creates nothing. It loads the roadmap into the compose state:
the text box locks and shows a card naming the roadmap and its next stage
(the stage's brief text stays server-side; no new read for it), the repo,
base and branch are the roadmap's own, and the pairings and companions stay
configurable. A clear control unloads it, restoring whatever brief text the
device draft held. With a roadmap loaded, **Start** creates the adopting
Conversation, applies the touched fields and kicks the adoption off;
**Save as draft** creates it without kicking off, as today's menu row did
minus the navigation into a start.

## Acceptance criteria

- [ ] The adopt dropdown appears under the box on the left only when
      abandoned roadmaps exist and the brief box is empty; its rows carry
      the roadmap, repo, next stage and non-default base.
- [ ] Loading a roadmap locks the box to a roadmap card, fixes repo and base,
      leaves pairings and companions live; clearing restores the typed
      draft.
- [ ] Start creates the adopting Conversation and kicks it off; Save as
      draft creates it still drafting — both navigating in.
- [ ] The New-conversation menu is gone, the button standing alone, and
      every mention of the menu in CONTEXT.md follows the move.
