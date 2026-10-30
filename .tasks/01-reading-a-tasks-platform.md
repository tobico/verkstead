# 01. Reading a task's platform

## What to build

A task line in `.tasks/TODO.md` declares the platform it wants the way a stage
line in `ROADMAP.md` does (ADR-0029, *Declaring what the work wants*):

```text
- [ ] 03: Title — [details](03-x.md) — on windows
```

The reading is shared and the judging is not. `declarations::read`, applied to
the tail `checklist::Entry::after` hands back for a backlog line, yields its
platform. A task line's platform is read **alone**: a task list has no
dependencies, so it has no all-or-nothing rule. A backlog that declares on some
lines and not others is still a backlog, `on windows` as a line's whole tail
declares (where on a stage line it does not), and a bare line wants no
platform. Give the backlog a small reading of its own that hands each entry's
platform over matched to `platform::Platform` (through the same three words
`declarations` matches, which are `PLATFORMS`/`matching` today), for the driver
in task 02 to use.

A word that is not one of the three is refused **when the backlog lands**: at
the `verkstead done` of the session that wrote it. That session is the planning
session of a stage or the breaking-down session, both of which wait on
`Landing::Arrived(.tasks/TODO.md)` in `runner::lacking`, beside where a
roadmap's `stages::misdeclared` refusal sits. The refusal names the task by its
label and lists the three platforms in the words `declarations::judge` refuses
a roadmap's in, made into a task's sentence. Nothing refuses at launch: a word
that got past this is read as no platform there.

Rewrite the `declarations` module docs' *Nothing here is a backlog's* paragraph
to say which part of this a backlog now shares.

## Acceptance criteria

- [ ] `- [ ] 03: Title — [details](03-x.md) — on windows` reads `windows`, `— On Windows` reads it too, and a bare line reads none, all as unit tests on the reading.
- [ ] A backlog declaring on one line only is read without refusal, and an in-progress-style annotation on a task line is not read as a declaration.
- [ ] A committed `.tasks/TODO.md` with `on freebsd` on task 03 makes `verkstead done` of a breaking-down session (and of a stage's planning session) exit non-zero with a sentence naming task 03, `freebsd` and `linux, macos, windows`; fixing the line and committing lets it through.
- [ ] The module docs say what of the reading a backlog shares, and that its judging is the landing's.
