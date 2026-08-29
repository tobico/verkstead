# 08. The repo details pane

## What to build

Repo cards become pressable, opening a read-only details pane at
`/settings/repos/:id` — the first slice that needs the server to say more
about a Repo than the list does.

One new UI endpoint answers for a single registered Repo with the facts the
pane shows (the bundle settled in the grilling):

- the resolved path and the default branch (what the card already shows)
- the branch list, local and remote-tracking, as the existing branches
  endpoint reads it
- how many Conversations are on the Repo — live and finished counted
  separately, where finished is Done or Closed
- the roadmaps in it waiting for adoption, as the abandoned-roadmaps read
  already finds them

`None`/404 for an id that is not registered, which the pane reads as the
repo being gone — a link followed after somebody took it away.

The pane draws those facts under the repo's name, with the card reading as
open while it is. Nothing here mutates anything; Remove arrives in task 09
and this pane is where it will sit.

## Acceptance criteria

- [ ] Pressing a repo card opens `/settings/repos/:id` showing path, default
      branch, the branch list, live and finished conversation counts, and
      any roadmaps waiting for adoption
- [ ] The endpoint answers 404-shaped for an unregistered id and the pane
      says the repo is gone rather than erroring
- [ ] The card reads as open while its pane is, like every other card
