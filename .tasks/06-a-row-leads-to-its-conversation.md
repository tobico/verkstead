# 06. A row leads to its Conversation

## What to build

Each stage **in flight** on the card leads to the Conversation working it: pressing
that row goes to that Conversation, while the card's head and its other rows still
open the roadmap's details pane, which is what the whole card opens today.

Which Conversation a stage is, is Verkstead's own record — the same rows that say
how far each stage of a roadmap got, which keep the roadmap and the label and drop
the Conversation the two belong to. It comes back, so that the list the card is
drawn from carries a Conversation per stage in flight and nothing for the rest.

- **A stage with no Conversation is not a press** — one that has not started, and
  one worked by hand or by the old tools, which the record holds no row for. It is
  a row that reads, like every row did before this.
- **A settled stage keeps its link** where the record has the Conversation: the
  stage is done and the Conversation is still where its pull request and its
  review are.
- **The card on the roadmap's own Timeline and the card on each stage's lead to the
  same Conversations**, being the one reading in two places.
- **A stage may have been attempted twice**, one Conversation closed and another
  started after it. The record already says which of two standings to believe for
  one label; the link follows the same answer, so the row leads to the Conversation
  the state is about.

The stage list is not part of what a shared Conversation shows, so a link out to a
Conversation from one of these rows reaches nothing the reader could not already
open.

## Acceptance criteria

- [ ] Pressing an in-flight stage's row goes to that stage's Conversation, and
      pressing the card's head or a row with no Conversation still opens the
      roadmap's details pane.
- [ ] A stage the record holds no Conversation for is not a press, and the card
      still opens the roadmap from it.
- [ ] Two cards drawn for one roadmap — on its own Timeline and on a stage's — lead
      to the same Conversations.
