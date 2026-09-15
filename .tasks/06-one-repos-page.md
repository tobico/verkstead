# 06. One Repos page

## What to build

Every registered Repo is its own card on the settings page today, opening a
pane of facts, branches, roadmaps waiting, a conflict picker and its own
binds, with Remove at the foot; and a plus on the heading opens a registration
form. All of that collapses into one **Repos** card, whose summary is how many
are registered, opening one pane that lists every Repo by name with a Remove
button beside each. A row shows the name and nothing else.

Remove asks once before it acts, the way closing an active run does, since it
is now one press in a list on a phone. A refused removal still says why inline,
in the words the pane uses today, and a live conversation on the Repo still
refuses it by name. Removing takes the Repo off the registry and touches
nothing on disk, as it does today.

Adding a Repo leaves the settings page: the plus and the registration pane go,
and Open repo and Create repo on the new conversation page, which share the
registration form, are the way in and are untouched. Every route under
`/settings/repos/` is no such page. The endpoint that served a single Repo's
pane goes with the pane, since nothing else reads it; the branches and pairings
reads the new conversation page uses stay.

The roadmaps-waiting list the pane showed is also reached from the new
conversation dropdown, so nothing is lost by dropping it here. The binds
section on the pane goes with the pane; task 07 takes the server side of
repo-scoped binds out behind it.

Tests: the repos web tests cover the count on the card, the list, a confirmed
removal, a refused one, and the retired routes; the server test for the single
Repo read goes with the endpoint. The design doc's settings block says each
Repo is a card with a pane and Remove on it.

## Acceptance criteria

- [ ] The settings page shows one Repos card with a count, and its pane lists every registered Repo by name with Remove beside each.
- [ ] Remove acts only after confirming, and a live conversation on the Repo still refuses it by name inline.
- [ ] No route under settings opens a single Repo or a registration form, and Open repo and Create repo on the new conversation page still register one.
