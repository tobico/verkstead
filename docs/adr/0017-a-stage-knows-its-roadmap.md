# A stage knows its roadmap

Builds on [ADR-0008](0008-pick-informs-artifacts-move.md): picking the roadmap
direction is what produces a committed roadmap, and every stage under it is
started from that document. What this revises is the rule
`crates/server/src/stages.rs` states in its own module doc — that nothing about
a roadmap is stored, so that nothing can come to disagree with the branch it is
read off. That rule stands for everything a roadmap *says*. It is reversed here
for one fact: **which roadmap a Conversation is a stage of**.

A roadmap stage's wrap-up settles, and Verkstead decides what to start next.
Until now it re-derived which roadmap from git at every settle: `git diff`
against the base commit under `docs/roadmaps/`, then a heuristic that looked for
a roadmap claiming the branch by annotation or by name. Where nothing claimed it,
the reading walked **every** roadmap the branch had touched, alphabetically,
skipping complete ones, and started the first unchecked stage it found.

That walk is the defect. A repository keeps its finished roadmaps — that is what
they are for — and a branch touches a second roadmap for ordinary reasons: a
deferral retired, a decision a later effort recorded corrected in passing. So a
stage whose own roadmap ran out handed over to an unrelated effort, in name
order, at the end of an unattended run, with nobody watching. The human reported
exactly that.

## The decision

**The roadmap a stage belongs to is a stored fact.** It is written once, at the
moment the stage's work is selected, and every carry-on reads only that roadmap.
Nothing is derived from git at wrap-up any more.

It lives in `stage_roadmaps (conversation_id, roadmap)`, a `STRICT` table beside
`stage_branches`, which is the store's own convention: there is no migration
machinery here, `conversations` is `STRICT` and left alone, so every new fact is
a table of its own keyed on the Conversation. Only the directory name under
`docs/roadmaps/` is kept. What the roadmap says — its boxes, its briefs — is
still the repository's and is read off the Worktree wherever it is wanted.

**The same grounds `stage_branches` already gives.** That table holds which
branch a stage stacks on, and its reason is written down: it is Verkstead's own
decision, taken once, at the moment the branch is made, and the repository never
records it. *Which roadmap this is a stage of* is the same kind of fact. The
repository records that a stage is in flight — the annotation in `ROADMAP.md`
says so — but a Conversation is Verkstead's, and which effort it belongs to was
chosen by a human adopting a roadmap, by the stage before it carrying on, or by
the branch that wrote the roadmap. None of those choices is in the repository.

**Written at three moments, once each.** The carry-on that starts the next stage
automatically; the adoption that starts one from *Continue a roadmap*; and the
Conversation that *wrote* a roadmap, at the moment Verkstead sees
`docs/roadmaps/` land on its branch — recorded there only if the branch created
exactly one roadmap directory, created meaning its `ROADMAP.md` is untracked or
added against the base commit.

**The roadmap-writing Conversation still auto-starts stage 01** of what it
wrote. Writing the roadmap was the selection.

**No recorded name means nothing starts**, and the three cases are told apart by
what else is stored rather than by anything read off the branch:

- A stage from before this change — a `stage_branches` row and no roadmap —
  says on its Timeline that it can be continued from the adoption menu. They
  are few and they are continued by hand.
- A roadmap-writing Conversation whose branch did not create exactly one
  roadmap says so, and starts nothing.
- An ordinary Conversation that edited a roadmap file in passing carries
  nothing on, silently. This case could previously start a stage.

**A recorded roadmap missing from the branch** — its directory renamed or
deleted, or its index emptied of numbered entries — is said on the Timeline and
starts nothing, the treatment a missing brief already got.

## What this costs, and why it is paid

The module doc's promise was that a reading of the Worktree cannot disagree with
the branch it is read off. A stored name can: a branch that renames its roadmap
directory leaves a record pointing at nothing. That is the case above, and it is
answered by saying so rather than by guessing — which is the whole point. The
alternative kept a heuristic that could start work nobody asked for, at the end
of a run nobody was watching, and a wrong guess is more expensive than a Notice.

## Considered Options

- **Keep deriving from git and tighten the heuristic.** Rejected: it stays a
  heuristic that can guess wrong, and the failure mode is unattended.
- **A minimal patch removing only the fall-through**, so that a branch no
  roadmap claims carries nothing on. Rejected: it leaves the alphabetical walk
  in place for every other shape.
- **A column on `conversations`.** Rejected by the store's own convention:
  that table is `STRICT` and there is no migration machinery.
- **Reusing the `adoptions` table.** Rejected: it has the same shape and means
  something else — a Draft that is *about to* adopt a roadmap, superseded the
  moment the stage starts.
- **Backfilling the stage's roadmap from its branch prefix at startup.**
  Rejected: a name guessed off a branch is the guess this removes.
- **Making the roadmap-writing Conversation's first stage manual too.**
  Rejected: writing the roadmap was the selection.
- **Recording the alphabetically first of several created roadmaps.** Rejected:
  that is the defect, stored.
- **Naming the other incomplete roadmaps in the completion Notice.** Rejected:
  the adoption menu already lists them.
- **A Timeline line for the ordinary Conversation that touched a roadmap in
  passing.** Rejected: there was never a roadmap of its own for the human to
  wonder about, and naming one it merely edited invites the confusion this
  removes.
- **Falling back to git where the recorded roadmap is missing from the branch.**
  Rejected for the same reason as everything above it.
