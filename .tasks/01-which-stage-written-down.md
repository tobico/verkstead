# 01. Which stage, written down

## What to build

The record that says which roadmap a Conversation is a stage of gains **which
stage of it**: the stage's label as the roadmap writes it — `05`, zero-padding
and all, the same form a declaration's `after 05` names it in and the same form
the stage entry already carries. Nothing derives it from a branch name; that is
the guess the record exists to stop.

**A column beside the roadmap rather than a table of its own**, which is the
human's choice and is what the store's convention has become: ADR-0017 wrote
that there is no migration machinery here, and there is now — `migrations.rs`
carries a column arriving as a one-time rewrite ten times over, `STRICT` tables
among them. So the column arrives the way those did, empty, and a database
written before this needs nothing else. Say in the column's own comment why it
is a column and not a table, because the ADR beside it says otherwise.

**Written at the moment a stage starts and by nothing else**, in the one
transaction that already records the roadmap and what the branch stands on — a
fact Verkstead decided once, on the grounds ADR-0017 gives. The three moments
are the three ADR-0017 names, and two of them are the same write: the carry-on
that starts the next stage, the adoption that starts one from *Continue a
roadmap*, and stage 01 of a roadmap just written, which is that roadmap
Conversation's own carry-on.

**The Conversation that wrote a roadmap gets no label.** It has a roadmap
recorded against it — writing the roadmap was the selection — and it is not a
stage of anything. So a row holding a roadmap and no label means one of two
things, and they are told apart by what is stored beside it rather than by
anything read off a branch, exactly as the carry-on already tells them apart: a
stage started before the label was recorded has a branch it stands on, and a
roadmap's own Conversation has the Roadmap direction.

The label is read back beside the roadmap, and **nothing reads it yet** — task
02 is the first thing to. What this task delivers is the fact, at every moment
one is made.

## Acceptance criteria

- [ ] A carried-on stage, an adopted one and stage 01 of a roadmap just written
      each have their roadmap and their label on the record, the label in the
      form the roadmap's own line writes it.
- [ ] The Conversation that wrote a roadmap has its roadmap and no label, and a
      stage from before this has neither.
- [ ] A database written before this opens with the column there and empty, and
      nothing about it reads differently.
- [ ] The label is read back beside the roadmap, and a row with a roadmap and no
      label is told apart from no row at all.
