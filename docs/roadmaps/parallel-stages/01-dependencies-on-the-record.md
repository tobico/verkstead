# 01. Dependencies on the record

## Goal

A roadmap's stage lines carry what each stage stands on and, where it has one,
its platform — `after 02, 03`, `no dependencies`, `on windows` — and Verkstead
reads them: the skills that write a stage line keep them, a roadmap that
declares badly is refused at the roadmap session's `verkstead done`, and the
roadmap's details pane shows each stage's dependencies and platform. **Nothing
schedules by them yet**: every roadmap still runs in order, and a roadmap
written before this is read exactly as it was.

## Decisions in force

- **[ADR-0021](../../adr/0021-parallel-stages.md), *A roadmap declares its
  dependencies***: on the stage's own line, after the link. Not in the brief
  and not in a block of its own.
- **`no dependencies` is the root's wording**, the human's own choice over
  `after nothing`. Every line of a declaring roadmap carries a declaration, and
  **a roadmap declaring on some lines and not others is refused**, because a
  bare line there cannot be told from a forgotten one.
- **A roadmap declaring nothing at all is one run in order**, and is not
  refused by anything: that is every roadmap written so far.
- **A cycle or an `after` naming no stage is refused**, not repaired and not
  run in order instead. In this stage that is the refusal at `verkstead done`
  for the session that wrote the roadmap; the other two refusals — a running
  roadmap starting nothing and saying why on the Timeline, and Continue a
  roadmap saying so at the press — are stage 04's, where there is a start to
  refuse.
- **A stage may only name stages of its own roadmap**, by their labels as
  written, zero-padding and all.
- **The platform is read, recorded and shown, and acted on by nothing.**
  Placing a stage on a device is a follow-up after cluster mode.
- **What follows the link already has a tenant**: the in-flight annotation
  `*(in progress: `branch`)*`, matched by the branch in backticks. A
  declaration and an annotation share the tail of one line, in either order,
  and neither reading may trip on the other.
- **And the declaration survives every skill that rewrites the tail.** Three
  skills write that end of a line: `staging` writes it, `next-stage` appends the
  annotation when it plans a stage, and `next-task` takes the annotation off
  again when it ticks the stage. The last two show the whole line in their own
  templates and would write a declaration away without meaning to — and a
  roadmap that has lost one line's declaration is the mixed roadmap refused
  above. So all three say the declaration stays, and every example line in them
  carries one.

## Proposed tasks (provisional)

1. **Reading a declaration** — the tail of a stage line yields its
   dependencies and its platform beside the annotation it already may hold.
   AC: `after 01, 03` reads as two labels; `no dependencies` as a root; a line
   with an annotation and a declaration reads both; a line with neither reads
   as undeclared.
2. **Judging a roadmap** — a roadmap is undeclared, declared, or refused with
   the reason in words a human can act on. AC: a mixed roadmap is refused
   naming a bare line; a cycle is refused naming the stages in it; an unknown
   label is refused naming the line and the label.
3. **The refusal at done** — the roadmap session's `verkstead done` refuses a
   roadmap that declares badly, the way it refuses one not committed. AC: the
   refusal exits non-zero and names the fault; an undeclared roadmap is
   accepted.
4. **The skills that write the line** — `staging`'s step 2 and roadmap template
   ask for a declaration on every line, in these words, and the prose dependency
   notes stay as the *why*; `next-stage`'s annotate step and `next-task`'s tick
   step both say the declaration stays where it is. AC: the skills' text
   assertions in `skills.rs` cover the wording; every template and example line
   in the three carries a declaration; a stage annotating itself in progress and
   ticking itself off leaves the line's declaration untouched.
5. **Shown on the roadmap** — the details pane shows each stage's
   dependencies and platform. AC: a declared stage names what it stands on; an
   undeclared roadmap looks as it does today.

## Re-verify at start

- `crates/server/src/checklist.rs` is still the one parser for `TODO.md` and
  `ROADMAP.md`, and `Entry::after` is still the tail after the link. A task
  list has no dependencies — *the order is the dependency* — so whatever is
  added must leave `TODO.md` alone.
- `ours` in `stages.rs` still matches an annotation by the branch in
  backticks, so a declaration holding no backticks cannot be mistaken for one.
- The platform names: cluster mode's ADR has a session told each device's OS.
  Use the names it uses, if it has landed or settled them, rather than
  inventing a second set.
- `done.rs` still checks a roadmap session against the committed roadmap, and
  is where a further refusal belongs.
- `next-stage`'s *Bring the roadmap's own score up to date* and `next-task`'s
  *tick the stage off* are where the tail is rewritten, and both spell the whole
  line out in an example. They are the ones to correct beside `staging`.
- **This roadmap's own lines will have lost declarations by the time you read
  it.** Every stage of it is planned on the installed server, whose `next-stage`
  writes the annotation over whatever the tail held, so put back what the lines
  above this one dropped — otherwise the roadmap is mixed, which is the very
  thing this stage refuses.
- `StageDocument` in `crates/render` is still what the details pane is fed,
  and the generated viewer types are regenerated by the test run.
