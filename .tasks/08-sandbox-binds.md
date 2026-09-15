# 08. Paths becomes Sandbox binds

## What to build

The Paths section is renamed **Sandbox binds**, promoting its one subsection to
the section, at `/settings/sandbox-binds`, with `/settings/paths` no such page.
The pane's explanatory paragraph is replaced by one line:

> Configures additional paths which are accessible from within the sandbox.

The lone subsection heading inside the pane goes, since the pane's title says
it. Everywhere the section said "binds every sandbox gets" it now says
"paths": the card's summary counts N paths, the zero state says there are no
paths, and the add form adds a path. The unseen warning on the card stays.

Tests: the paths web tests follow the rename and the wording, and the routes
test learns the new slug and the retired one. The design doc's settings block
and `docs/development.md` name the section Paths.

## Acceptance criteria

- [ ] The card reads Sandbox binds with N paths under it, the empty pane says there are no paths, and the add form adds a path.
- [ ] `/settings/sandbox-binds` opens the pane and `/settings/paths` is no such page.
- [ ] The pane holds the one line above, the rows and the add form, with no subsection heading.
