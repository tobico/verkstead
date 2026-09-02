# 03. The Repos' form browses

## What to build

The repo-registration path field adopts the component from task 02, in the
**watched** scope: a Repo may only be registered from inside a Watched Path, so
the dropdown opens on the watched roots and never offers anything the server
would refuse for being outside them.

This field gets the one git-aware behaviour: an entry task 01 marked as a
**repository** draws marked — it is what this field is looking for — and is a
leaf, not descended into. Tapping it writes the path into the field like any
other row.

Registration itself is untouched: Add sends the path exactly as today, and the
server's admission and refusals (not absolute, missing, outside the watched
paths, not a repository, and the rest) stay the only thing that decides what a
submitted path does.

The Repos' form's module header carries the same "typed rather than picked"
sentence as the Paths section's did; rewrite it the same way.

## Acceptance criteria

- [ ] With the field empty the dropdown offers the watched roots; browsing down
      to a repository fills the field with its path.
- [ ] A repository entry draws marked and does not open; plain directories
      drill as in task 02.
- [ ] Registering through a browsed path behaves exactly as a typed one, and
      the rewritten module header lands.
