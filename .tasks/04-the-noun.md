# 04. The noun

## What to build

The noun is the plugin's. The viewer draws the Forge's declared noun wherever a
Repo on that Forge is drawn:
- the pinned card;
- the pull request pane;
- the Target field's placeholder;
- the Share-to row;
- the push notification the server sends.

The generated types carry the noun per Repo, next to task 02's Forge. A Repo
with no Forge keeps the *No forge* wording and needs no noun.

CONTEXT.md's *pull request* entry becomes the word for the thing in the
documents, saying the screen uses what the Forge declares. Sweep the viewer for
every on-screen *pull request* tied to a Repo. Leave alone the generic ones not
tied to a Repo, such as documentation copy about Verkstead itself.

## Acceptance criteria

- [ ] A Forge declaring *merge request* reads so on the card, the pane, the
      Target placeholder, the Share row and the notification, with articles
      and plurals right.
- [ ] A Repo on GitHub reads exactly as before.
- [ ] CONTEXT.md says the screen says what the Forge says.
