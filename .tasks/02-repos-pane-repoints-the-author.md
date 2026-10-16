# 02. The Repos pane shows and repoints the Author

## What to build

The settings page's Repos pane (the list of registered Repos with a Remove on
each row) gains the Author. Once more than one Author is configured, each row
shows its Author's display name (falling back to the git name, as the Git
section's switcher labels them) and a control to change it; with one Author
nothing is drawn. A row whose Author is unpicked (`author_unpicked`) or gone
says so and offers the same control.

A new endpoint repoints a Repo to another Author's uuid. The server refuses:

- a uuid that is not configured;
- **while a Conversation on the Repo is neither Done nor Closed** (the
  `work_on_repo` live reading unregistering already uses) — the refusal **names
  the Conversation** (one of them, or all), and the row says so under it, the
  way an unregistering refused for live work is shown;
- **while the Repo is a Companion Repo of a Conversation that is neither Done
  nor Closed** — named the same way. (Decided in planning: a companion's
  Author must not change under the work that admitted it. Task 04 adds the
  matching check at the cut.)

A repoint that lands is recorded in the store's Repo–Author association
(`repo_authors`), replacing the row, and the next session on the Repo commits
and carries `GH_TOKEN` as the new Author, since the sandbox reads the Author at
launch. CONTEXT.md's **Author** term already says the Repos pane is where it is
changed; keep it true.

## Acceptance criteria

- [ ] A repoint on a Repo with a live Conversation is refused and the row names that Conversation; likewise for a Repo that is a live Conversation's companion.
- [ ] A repoint on an idle Repo lands, the row redraws with the new Author, and the next session on it carries the new Author's `user.email` and `GH_TOKEN` and nothing of the old one's.
- [ ] With one Author the pane draws no Author and no control.
