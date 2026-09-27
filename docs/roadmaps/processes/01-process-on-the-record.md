# 01. Process on the record

## Goal

Every Conversation has a Process, and the composer asks for it. A new draft on
the compose page or a saved Draft's composer shows a **Process** picker between
Repo and the Pairings, offering Develop and nothing else yet, defaulting to
Develop and switchable while the Conversation drafts; Start freezes it with the
Pairings, and from then on the Brief's setup facts and the details pane say
which Process the Conversation is. Every Conversation from before reads as
Develop, except one that adopted a pull request, which reads as Review. Nothing
about how Develop runs changes.

## Decisions in force

- **[ADR-0020](../../adr/0020-a-conversation-has-a-process.md), *Process
  beside Direction*** is the shape: a fact of the Conversation's, picked before
  anything runs, frozen at Start, kept in a 1:1 side table keyed by the
  Conversation with the `conversations` table left alone — the pattern
  `directions`, `adoptions` and `pull_request_adoptions` already follow, with a
  stored-word pair the way `Lifecycle` has one. A column was not chosen because
  the table is `STRICT` and every per-Conversation scalar since has gone beside
  it rather than into it.
- **The enum is `Lifecycle`'s pair, not `Direction`'s home**: a store enum in
  `crates/store/src/conversations.rs` with its stored words, and a render enum
  in `crates/render/src/conversations.rs` with the viewer's, which is what
  emits the TypeScript — `verkstead-render` owns that export, so a render enum
  is carried across the wire exactly as a schema one is. Not the schema crate:
  `Direction` is there because it rides a Question Set, as a field of
  `Proposal`, and that crate is the Set and Response grammar the agents write
  and is compiled to wasm for the browser. A Process is on no Set, and
  `Lifecycle` — the per-Conversation fact this is shaped like — is deliberately
  not there either.
- **All five variants from this stage**, even though only Develop can start:
  the store reads and writes every one, and what a stage after this adds is a
  start path and a row on the picker, never a variant.
- **Conversations from before read Develop; one holding a pull-request
  adoption reads Review.** Read rather than backfilled, so nothing is written
  into old rows: the reading is the same rule whichever a row lacks.
- **Develop is the default and is not remembered per Repo**, unlike the
  Pairings (CONTEXT.md, **Process**). The compose page holds the pick on the
  device beside the other fields and sends it at create only where the human
  touched it, exactly as a role picker left on the prefill sends nothing.
- **The picker offers a Process only once its stage has landed**, as an agent
  type is offered only once it can launch the real thing. One row for now, and
  the row list is the one place a later stage adds to.
- **Drawn on the Brief's setup facts, nowhere else.** The sidebar row and the
  card keep the state word. *Amended while planning this stage*: beside the
  Repo, the branch and the base rather than beside the Pairings — those facts
  come in two halves and the Pairings are in the one a Share does not draw,
  being about the machine. The setup facts and the details pane are the same
  place: that pane is where they are summarised.
- **Switching is a Draft's freedom**, refused from the moment a worktree
  exists, on the same rule and with the same refusal shape as switching the
  Repo. Frozen at Start beside the Pairings — which is the refusal and nothing
  more (*amended while planning this stage*): the Pairings freeze because every
  field endpoint refuses once a worktree row exists, and the start transaction
  writes no Pairing, so nothing is written for a Process either. A row means
  somebody picked; no row means the reading stands.
- **A Draft holding a pull-request adoption draws the control disabled, reading
  Review** (*settled while planning this stage*), as the Repo picker reads
  disabled once it is settled: that is what the Conversation is, and Review is
  not pickable until stage 05.

## Proposed tasks (provisional)

1. **The `Process` enum and its table** — the store enum with its stored
   words, the render enum with the viewer's, the side table, a read that
   answers Develop or Review where no row is, and the loaded Conversation
   carrying it. AC: a fresh Conversation reads
   Develop; one with a pull-request adoption and no row reads Review; a written
   Process reads back.
2. **The API** — a per-field endpoint beside the Pairing ones that sets a
   Draft's Process, refused past drafting and for a Process whose stage has
   not landed; the Conversation view and the setup facts carry the word. AC:
   the composer's replay can set it; a non-Draft is refused by name; the view
   says Develop.
3. **The picker, drawn twice** — a `Process` control in the shared setup row
   between Repo and the Pairings, on the compose page over device state and on
   the saved Draft's composer over the record, labelled *Process*, one row. AC:
   the compose page shows Develop; a saved Draft can be switched and refused
   past drafting; a pick left on the default sends nothing at create.
4. **Where it is read back** — the Brief's setup facts and the details pane say
   the Process; the sidebar and card do not. AC: a frozen Brief shows *Develop*
   beside the Pairings; the row label is unchanged.

## Re-verify at start

- `directions` is still a 1:1 side table with a stored-word pair in
  `crates/store/src/conversations.rs`, and the `conversations` table is still
  the one left alone.
- The setup row is still shared between the compose page and the saved Draft's
  composer through `web/src/workbench/Setup.tsx`, drawn twice.
- The compose page's create is still a replay through per-field endpoints with
  no batched create, so a new field is a new endpoint and a new replay step.
- `Lifecycle` is still a pair — a store enum with its stored words in
  `crates/store/src/conversations.rs`, and a render enum with the viewer's in
  `crates/render/src/conversations.rs` — and `Direction` is still the schema
  crate's, reached through a Question Set's `Proposal`.
- The generated TypeScript types are still checked up to date in CI, and
  `verkstead-render` is still what emits them, so a new render enum regenerates
  them.
