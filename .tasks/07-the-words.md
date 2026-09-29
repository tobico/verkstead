# 07. The words

## What to build

What the project says about itself, brought up to a roadmap that schedules.

- **`CONTEXT.md`'s Stage** — *started by the Stage before it settling*, and
  *Nothing schedules by any of it yet*. Every stage whose dependencies have
  **settled** starts, up to a limit per roadmap; a stage waiting on the human or
  waiting to join takes a place; a halt before the join holds up only what stands
  on the halted stage; an undeclared roadmap runs strictly in order, each stage
  standing on the one before it.
- **`CONTEXT.md`'s Adopt** — *One Stage is the whole of what adopting starts*.
  The press starts every ready stage, up to the limit.
- **`CONTEXT.md`'s Notice** — which Stage it started becomes which Stages, and
  the roadmap that declares badly joins the list of what a Notice reports.
- **The `next-stage` skill's *no other plan in flight to check for*.** A sibling
  stage of the same roadmap may well be in flight now, and it is still nothing
  this session checks for or acts on — the sentence has to stay true for a
  reason it did not have before.
- **`docs/design/verkstead.md`'s *the next stage starts only after wrap-up
  completes***, and *Stages always stack* where the ordering shows through it.
  **Refined rather than rewritten**, in the document's own *refined <date>,
  building <stage>* form, the way every other decision there is refined as the
  stage that changes it lands.

## Acceptance criteria

- [ ] The glossary's **Stage**, **Adopt** and **Notice** say what the scheduler
      does, in the glossary's own voice and with ADR-0021 referenced rather than
      restated.
- [ ] The `next-stage` skill's sentence about nothing else in flight still reads
      true with stages worked side by side.
- [ ] The design document is refined in its *refined <date>, building <stage>*
      form, with the superseded wording left standing in front of the refinement.
