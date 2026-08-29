# 05. Resume into the menu, and the alert modal

## What to build

Resume stops being a block at the foot of the Timeline and becomes the first
row of the conversation actions menu — above Stop, because it is the one "go"
action among stops and closes. The rows are one factory drawn in two menus, so
Resume arrives in both: the StatusButton's menu and the sidebar's right-click
context menu. It is offered exactly where the server says it is worth offering
(`ready_to_resume`), and its press is the same recompute-and-start it always
was.

Refusals change shape for every row, not just Resume's: instead of
`console.error`, a refused press — and a press whose request failed outright —
opens an alert modal saying the refusal's sentence, built on the existing Modal
component (a heading, the sentence, one way out). Resume's refusal sentences
are the human-facing guidance already written for the foot block; the other
rows' sentences are the ones they log today. This was settled deliberately
over the menu's previous refusals-are-console-noise stance.

The foot-of-pane Resume block goes entirely — heading, button, refusal lines
and note — and so does the live-session "Agent run" strip held against the
pane's foot: the StatusButton now says what is running, and the session's card
is at the record's end where the pane already opens. The out-of-window
`resets` sentence moved to the StatusButton's second line in task 04, so
nothing here re-homes it.

## Acceptance criteria

- [ ] Resume is the first row in both menus, drawn on `ready_to_resume`, and resuming works from each
- [ ] Any row's refusal or network error opens the alert modal with the refusal's sentence; nothing goes only to the console
- [ ] The foot Resume block and the live-session strip are gone from the Timeline pane
- [ ] Web tests cover the Resume row in both menus and the modal on a refused press
