# 01. The role table, and the roles a Process draws

## What to build

One place says, per Process, which roles it uses — Grilling, Implementation,
Review — which of them may be picked away, and whether its control is a
dropdown or a panel. It goes in the module that already says what each Process
is called and which of them the picker offers, that being the one per-Process
place the web has; ADR-0020 carries the table itself, and this is that table
written down where the web reads it.

    Develop             Grilling, Implementation, Review (or none)   panel
    Investigate         Implementation                               dropdown
    Review              Implementation, Review                       panel
    Tinker              Implementation, Review (or none)             panel
    Fix Merge Issues    Implementation                               dropdown

Both composers then draw a picker per role the Process uses, rather than always
three. Today the Grilling picker is hidden over a draft holding a pull request
by an explicit test for the held pull request, in both of them. That test goes:
a Conversation holding a pull request reads its Process as **Review**, and
Review does not use Grilling, so the table says it once where two pages said it
twice. The outcome for a held pull request is exactly what it is today.

And the title under an inert Start names the roles that Process has. It reads
*every role picked and working* now, with a *both roles* variant already written
for the held pull request — that variant becomes the table's too, and a Process
using one role says one role. The compose page's own readiness, which decides
how its press behaves, waits on exactly the roles the table names.

Nothing about the drawing moves yet: the row is still the flat dropdowns it is
today, each labelled by its role. What changes is which of them are drawn, and
what the start says it is waiting on.

## Acceptance criteria

- [ ] A Process answers which roles it uses, which may be picked away, and which
      shape its control takes, in the one place both composers and the readiness
      text read — Develop answering all three roles and a panel.
- [ ] A draft holding a pull request draws Implementation and Review and no
      Grilling because its Process is Review, with no test for a held pull
      request left around the drawing on either composer.
- [ ] The title under an inert Start names the roles that Process uses, on the
      Conversation's composer and the compose page both, and the compose page
      waits on exactly those roles.
- [ ] Exercised over a Process nothing offers yet: a record reading a one-role
      Process draws one picker and waits on one role.
