# 03. A tap opens a picture at full size

## What to build

A third of a screen is small for a diagram. A press on a picture in the Diff —
either side of a pair — opens that picture over the page in the modal the UI
already has, drawn at its own size and no larger than the window, on the same
chequerboard, with its side's word and the file's path beside it. It closes
every way a modal closes — Escape, a press on the backdrop, its own control —
and focus returns to the picture that opened it. The box in the Diff keeps its
place; nothing in the page moves. The bytes are the ones already fetched, so
nothing is fetched again.

It works wherever task 02 draws a picture: the Set sheet, the commit pane, and
so the share once task 04 lands. A picture that failed to fetch — hunks or a
line standing where it would be — has nothing to open, and is not a press.

## Acceptance criteria

- [ ] A press on a picture, in the Set sheet and in the commit pane, opens it at full size in the modal with its word and path, and the Diff behind it is unchanged.
- [ ] The modal closes on Escape, on the backdrop and on its control, returning focus to the picture that opened it; nothing is fetched to open it.
- [ ] A picture that was not drawn opens nothing.
- [ ] The viewer's suite covers the opening, each way of closing, and the no-op.
