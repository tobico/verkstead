# 02. Pairing choice

## What to build

Choosing who runs a Conversation's sessions becomes choosing a
**profile-and-model pairing**. The grilling choice and the implementation
choice each hold a profile *and* one of that profile's listed models, and the
launched session runs with the paired model rather than anything the profile
alone says.

Every picker that offers profiles offers pairings instead, as **one flat
list** — a row per profile–model combination, labelled `profile — model`, one
tap to choose. (A two-stage profile-then-model picker was considered and
rejected: it scales better but costs a tap every time, and the counts stay
small.) This covers the grilling picker, the implementation picker, and the
manual task's "Run it as" picker, which keeps prefilling from the
conversation's implementation pairing and otherwise demands an explicit pick —
there is no default model anywhere.

Both pairings **lock when grilling starts**. Today the server refuses editing
the branch, base and brief outside drafting but lets profile choice through at
any time; this task closes that gap with a refusal of the same shape, so a
choose call after grilling starts is refused and the UI reads that refusal.
(Settled deliberately: the alternative — leaving the implementation pairing
changeable until implementation starts — was offered and declined.)

Readiness to grill means both pairings are complete, model included. A
conversation that chose profiles before this change and is already past
drafting keeps working: it launches with the model its profile carried at the
time. One still drafting with a bare profile counts as unpaired and asks
again.

## Acceptance criteria

- [ ] All three pickers offer every profile–model pairing as one flat row,
      and a chosen pairing's model is what the session launches with
- [ ] Choosing either pairing after grilling starts is refused by the server
      and surfaced by the UI; while drafting both remain changeable
- [ ] Start-grilling readiness requires both pairings complete, and a
      pre-existing conversation past drafting still launches with its
      profile's former model
