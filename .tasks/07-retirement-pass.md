# 07. Retirement pass

## What to build

The switch-over. Everything before this makes the pipeline run; this is where it
becomes the thing actually used, and roadrunner and the tobico-scripts wrappers
stop being.

**Take a real repository through it end to end.** Not a fixture and not this
one's tests: a repository with real work waiting, driven from the workbench from
a Brief through grilling, a Direction, a backlog worked to empty, a finish that
opens its own PR, and a wrap-up that settles. This is the task where the
unattended pipeline meets a machine it was not written against, and what it
turns up is the work: fix what is broken, and write down what is merely
surprising.

**Document the switch-over.** What Verkstead replaces, and how a day's work runs
through it now — enough that starting a piece of work does not mean remembering
which of three tools used to do which part. It belongs in this repository's
`docs/`, beside what is already there.

**Deprecate what it replaces.** roadrunner and the tobico-scripts wrappers say
in their own repositories that they are deprecated and what replaces them.
Those are other repositories, so this part lands outside this branch — the work
here is doing it, and the PR carries only the note that it was done.

The adoption is the point of the stage rather than a footnote to it: the roadmap
says Verkstead replaces them for daily work when stage 04 lands, and a pipeline
nobody has run their own work through is not finished.

## Acceptance criteria

- [ ] One real repository has been taken from Brief to settled wrap-up inside
      Verkstead, and what that turned up is either fixed or written down.
- [ ] `docs/` says what Verkstead replaces and how a day's work runs through it.
- [ ] roadrunner and the tobico-scripts wrappers are marked deprecated in their
      own repositories, naming what replaces them.
- [ ] Anything the end-to-end run found and did not fix is recorded where the
      stage that would fix it will see it — a stage 05 note, or an issue.
