# 03. Mobile reopen fix

## What to build

On narrow screens, a conversation that is already selected cannot be reopened:
the back button flips the visible pane to the conversations list but leaves
the URL at `/conversations/{id}`, so tapping the same card navigates to the
URL already current, the selection signal never changes, and the pane-choosing
effect never runs.

Fix it by keeping the URL and the pane in agreement: going back to the list
clears the selection from the URL (so the pane effect drives the change), and
re-selecting the same conversation then genuinely changes the selection and
pages forward. Preserve the existing design intent that the URL drives which
pane shows, rather than special-casing the click handler.

## Acceptance criteria

- [ ] On a narrow viewport: open a conversation, go back, tap the same card —
      the conversation pane returns
- [ ] Opening a different conversation from the list still works, and the
      details-pane selection resets as it does today
- [ ] Wide-viewport behaviour is unchanged
