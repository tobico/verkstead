# 04. Companions of another Author are refused

## What to build

One sandbox is one environment — one `GH_TOKEN`, one git author — so a Companion
Repo under a different Author than the Conversation's own Repo is refused, in
either mode (read-only or read-write; read-only companions of another Author
were turned down in ADR-0025). Three places, all naming the companion and both
Authors by display name:

- **The composer's add.** The one companion endpoint already refuses the
  Conversation's own Repo (`OwnRepo`) and a duplicate (`AlreadyAdded`); the
  Author check sits beside them as a new outcome.
- **Switching a draft's Repo.** `switch_repo` keeps every companion; a switch
  onto a Repo whose Author differs from a companion's is **refused**, naming the
  companion and both Authors. (Decided in planning: refuse rather than drop.)
- **The cut.** A companion's Author can still change by a hand edit (and before
  task 02, by a repoint), so every cut that takes companions — the grilling cut,
  the adoption cut, and a stage's carry-on inheriting its predecessor's set —
  checks them again and refuses (or, for the carry-on, halts with a Notice) by
  name. A set whose Authors all still match is unaffected.

Compare Authors as `Config::author_of` resolves them, so a Repo with no Author
recorded where there is one Author matches it.

## Acceptance criteria

- [ ] The composer's add of a Repo under another Author is refused with the sentence, in both modes; the same Repo under the same Author is admitted.
- [ ] Switching a draft with a companion onto a Repo of another Author is refused naming them; onto one of the same Author it lands.
- [ ] A companion repointed by hand after admission makes the grilling cut refuse and a stage's carry-on halt, naming the companion and both Authors.
