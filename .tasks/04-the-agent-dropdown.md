# 04. The Agent dropdown, where a Process uses one role

## What to build

Where the role table says one role, the Agent control is not a panel at all: it
is the flat Pairing dropdown the role pickers already are, labelled **Agent**,
offering the Pairings and wired to the Implementation role. No trigger and no
panel — the panel is never drawn under a one-role Process. One control in two
shapes, which is what the table's third column is for, so a later stage adds a
row rather than a branch.

On both composers, and on the same ground as the pickers inside the panel: the
per-Repo memory is keyed by role, and Implementation is Implementation whichever
shape asks for it. On a Conversation the pick saves itself as it is touched; on
the compose page it is a field of the draft the device holds, replayed at
create.

No Process that uses one role is offered yet — Investigate arrives in stage 04
and Fix Merge Issues in 06 — so this is exercised against the table, over a
record whose Process reads one of them. The wire carries all five Processes and
the picker already draws a chosen Process it cannot offer, so there is a row to
stand this on without either stage having landed.

The readiness text names that one role, which task 01 already made the table's.

## Acceptance criteria

- [ ] A draft whose Process uses one role draws a dropdown labelled *Agent*
      offering the Pairings, on both composers, with no Agent panel drawn
      anywhere on the page.
- [ ] Picking through it settles the Implementation role and nothing else,
      prefilled from the per-Repo memory exactly as the picker inside the panel
      is, and replayed at create on the compose page.
- [ ] The title under an inert Start names that one role.
