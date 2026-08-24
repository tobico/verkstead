# 12. The remaining settings modals

## What to build

The settings page's other two inline forms move onto the modal component:

- **Add repo:** a button opens the register-a-repo form in a modal; the
  repo list stays on the page.
- **GitHub and git author:** the inline form becomes a summary on the page —
  the token's state (ending, when saved, or the unconfigured warning) and
  the author name and email — with an edit button opening one modal holding
  both sections and their single Save, exactly the shape the form has
  today, replace/clear included. Warnings that should reach someone not
  editing (no token, missing author) stay visible on the page summary.

After this task the settings page reads as summaries, lists and buttons,
with every form in a modal.

## Acceptance criteria

- [ ] Registering a repo runs in a modal; refusals render inside it
- [ ] The page shows token state and author as a summary with an edit button
- [ ] The modal saves token and author together as today, and the summary
      reflects a save immediately
