# 04. The Git section's switcher and Add author

## What to build

The settings page's Git section edits any Author, and adds one.

**The settings view carries every Author**: uuid, display name, git name,
email, and the token's saved state (last four characters, when it was written,
and the account GitHub verified it as). The save is per Author, addressed by
uuid. A save naming a uuid that is not in the list is a new Author, written at
the end of the list.

**The section keeps its shape.** A dropdown at its head picks which Author the
fields are about. The fields are display name (new here, optional, with the git
name as its placeholder), git name, email, and the write-only token with its
last four characters and verified account. **Add author** sits at the dropdown's
foot as an action row behind a rule, the way the Repo dropdown carries **Open
repo** and **Create repo** (the `actions` rows in `web/src/picking.tsx`).
Pressing it opens the same fields over a fresh uuid, which is saved like any
other. Each row's label is the display name, falling back to the git name.

**The dropdown and Add author are drawn only once there are two Authors.**
With one, the pane reads as it does today, so there is no way to add the
second from the page while only one exists. Decide where **Add author** lives
with one Author, which the brief leaves open. Keep it as quiet as the
*with one Author nothing changes* rule allows: a plain action under the fields
is the likely answer, and was put to the human as a deferred Q5 (Set 1209):
follow their answer where it reached this session. Check `docs/copy.md` for
the words.

**The token edit and its verification are per Author**: on save, naming the
login and any scope it lacks, as today. The cards beside the pane (warnings
about a missing token or author) speak about the Author they concern, and say
nothing more than today with one Author.

Removing an Author is stage 02's, and not built here.

**The wizard's git step** saves the one Author through the same save and is
otherwise unchanged.

## Acceptance criteria

- [ ] With one Author, the pane reads as it does today, with no dropdown drawn.
- [ ] Adding a second Author draws the dropdown with both, the display name
      falling back to the git name. Saving a token under the second verifies it
      and names that token's login, and the first Author's token is untouched.
- [ ] Switching the dropdown with unsaved edits does not carry them onto the
      other Author.
- [ ] The wizard's git step saves the one Author and is otherwise unchanged.
- [ ] The web suite covers the one-Author and two-Author panes.
