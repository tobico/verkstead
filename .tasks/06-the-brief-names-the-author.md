# 06. The Brief names the Author

## What to build

The Brief's record shows the Repo as a fact. Once more than one Author is
configured, the Author's display name (falling back to the git name, the label
the Git section and the Repos pane use) sits beside the Repo there; with one
Author nothing is drawn. The Conversation's Repo on the wire already carries
its Author's uuid (`RepoEntry.author`), and the settings view carries every
Author, so this is the viewer's join — the way the Repos pane row from task 02
draws it. A Repo whose Author is gone or unpicked says so rather than drawing
nothing.

## Acceptance criteria

- [ ] The Brief of a Conversation on a Repo under the second Author names that Author beside the Repo.
- [ ] With one Author configured the Brief draws no Author.
- [ ] A Repo whose Author is gone shows that on the Brief rather than a blank.
