# 02. A roadmap that declares badly, refused at done

## What to build

One judgement over a whole `ROADMAP.md`, and its first caller: the roadmap
session's `verkstead done`.

The judgement answers one of three things.

- **Undeclared** — not one line declares. The roadmap runs strictly in order,
  exactly as before, and nothing refuses it. That is every roadmap written so
  far, and reading silence as *start everything at once* would start stages on
  top of work they were written to follow.
- **Declared** — every line declares, and the judgement carries the graph:
  which stage stands on which, and the platform each wants.
- **Refused** — with the fault in words a human can act on.

Four faults, each named where it is:

- **Mixed** — some lines declare and some do not, named by a line that does
  not. A bare line in a declaring roadmap cannot be told from a forgotten one,
  which is the whole reason all-or-nothing is the rule.
- **An `after` naming no stage of this roadmap**, named by the line and the
  label it named. Labels match as written, so `after 1` against a stage
  labelled `01` is this fault rather than a match.
- **A cycle**, named by the stages in it. A stage naming itself is a cycle of
  one.
- **A platform that is not `linux`, `macos` or `windows`**, named by the line
  and the word.

Refused rather than repaired, and never run in order instead: falling back to
running in order runs a roadmap in a way nobody wrote down.

**Where it is refused, in this stage, is the roadmap's own session at
`verkstead done`** — beside the two refusals already waiting there, that the
branch has written no roadmap and that the roadmap is not committed. It exits
non-zero, says the fault on stderr, and the session stays alive to put it
right and signal again in the same turn. That check lives with the reading of
a landed roadmap rather than in the Done route itself, which hands it the
question.

**The other two callers are stage 04's** — a running roadmap that can start
nothing saying why on the Timeline, and Continue a roadmap saying so at the
press. Neither is built here, because neither has a start to refuse yet. So
the judgement goes somewhere both can reach, and the sentences it produces are
the ones they will show.

## Acceptance criteria

- [ ] A roadmap declaring on some lines and not on others is refused, naming a
      line that does not declare; one declaring on no line at all is accepted
      as undeclared.
- [ ] A cycle is refused naming the stages in it, an `after` naming no stage
      of the roadmap is refused naming the line and the label, and an
      unrecognised platform word is refused naming the line and the word.
- [ ] `verkstead done` in a roadmap session exits non-zero on each fault and
      says it, the way it already says the roadmap is not committed; a
      well-declared roadmap and an undeclared one are both accepted.
