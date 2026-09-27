# 01. The declaration, read and shown

## What to build

The tail of a stage line in `ROADMAP.md` — everything after the link to the
brief — is read for two things beside the in-progress annotation it may
already hold: **what the stage stands on**, and **the platform it wants**. And
what is read is shown: each stage's section in the roadmap's details pane says
what it stands on and names its platform where it has one.

The grammar is the roadmap's own, as ADR-0021 settles it:

- **`after 01, 03`** names other stages of the same roadmap, by their labels
  **as the roadmap writes them, zero-padding and all**. Matching `after 1`
  against a stage labelled `01` is not this task's business to forgive — the
  reading is faithful and the judging is task 02's.
- **`no dependencies`** is the root's wording. The human chose it over `after
  nothing`.
- **`on linux`, `on macos`, `on windows`** is the platform, and those three
  words are the whole set. A word that is not one of them still reads as *a
  platform naming that word*, so that task 02's judgement can refuse it by
  name rather than quietly seeing nothing there.

**A declaration and the annotation share one tail, in either order, and
neither reading may trip on the other.** The annotation is matched by the
branch in backticks and a declaration holds none, which is what keeps the two
apart — so the stage in flight is still found by its branch whichever way
round the line was written.

**A line carrying neither `after` nor `no dependencies` is undeclared**,
whatever else its tail holds. A platform on its own does not declare: `on
windows` alone would let a forgotten declaration hide behind a platform, which
is the thing the all-or-nothing rule exists to catch.

**`.tasks/TODO.md` is left exactly as it is.** A backlog has no dependencies —
the order is the dependency — and nothing added here may change how one of its
lines reads.

`CONTEXT.md`'s **Stage** entry describes a stage whose line carries no
declaration, and gains a sentence saying it does: read, recorded and shown,
and acted on by nothing yet. The entry's *started by the Stage before it
settling* is stage 04's to revise, not this task's.

Two things worth knowing before choosing names. `Entry::after` in the one
checklist parser already means *the raw tail after the link*, which collides
with the declaration's own `after` — so the new reading wants a name of its
own. And cluster mode, which is unmerged work on another branch, carries a
device's OS as a free-text display string (*Linux*, *macOS*, *Windows*, *Linux
(WSL)*) for drawing an icon beside a hostname; that is a different thing from
the three words matched here, and neither should be made to serve the other.

## Acceptance criteria

- [ ] `after 01, 03` reads as the two labels as written, `no dependencies` as
      a root, and `on windows` as a platform; a line with none of them reads
      as undeclared, and one whose only tail is `on windows` reads as
      undeclared with a platform.
- [ ] A declaration and `*(in progress: `branch`)*` in either order both read,
      and the stage in flight is still found by its branch.
- [ ] A declared stage's section in the roadmap's details pane names what it
      stands on and its platform; an undeclared roadmap's pane looks exactly
      as it does today, and a backlog reads exactly as it did.
- [ ] `CONTEXT.md`'s **Stage** entry says the stage's line declares what it
      stands on and may name a platform.
