# 11. A modal component, proven on profiles

## What to build

A shared modal component, and the agent-profile form as its first tenant.

- **The component** wraps the native dialog element (`showModal`), settled
  for its built-in Escape, backdrop and focus handling, and is styled like
  the existing confirm sheet: dimmed backdrop, bottom sheet on phones,
  centred card on wide screens. The two copy-pasted confirm sheets (archive
  and submit-with-unanswered) are candidates to move onto it if that stays
  a refactor rather than a redesign — the component's contract comes first.
- **Profiles:** the always-visible inline add/edit form on the settings page
  becomes a modal. An Add-profile button opens it empty; each row's edit
  action opens it prefilled, one form for both as today. Saving, refusal
  copy and removal behaviour are unchanged; the list is what remains on the
  page.

## Acceptance criteria

- [ ] Add and edit both run in the modal, prefilled on edit, and the inline
      form is gone from the page
- [ ] Escape and a backdrop click close the modal without saving
- [ ] Refusals render inside the modal and keep it open
