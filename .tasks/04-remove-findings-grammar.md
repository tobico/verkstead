# 04. Remove the findings grammar from the schema

## What to build

With nothing left reading it (tasks 02 and 03), the `review` block leaves the
Question Set grammar entirely: the `review` field on a Set, the findings types
(`Review`, `Finding`, `Decided`) and the validation that held `fix` and `split`
to Options the Set actually offered all go from the schema crate, and the
server and CLI compile without them.

A review's Set is from here on indistinguishable from any other ask — that is
the point of the change. Compatibility is one-way and cheap: a Set that still
arrives carrying a `review` key is accepted and the key ignored, so nothing in
flight breaks; nothing ever emits one again.

The generated TypeScript API types follow the schema, so regenerate them and
sweep the workbench for anything that named the block.

## Acceptance criteria

- [ ] The schema crate exposes no findings types and a Question Set has no
      `review` field; server and CLI build and their tests pass.
- [ ] A Set sent with a stray `review` key is accepted, the key ignored — shown
      by a test.
- [ ] The generated web API types no longer mention the block, and schema, CLI
      and server tests that spoke the old grammar are updated rather than
      deleted where they still test something.
