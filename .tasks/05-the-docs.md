# 05. The docs

## What to build

The documentation catches up with what the four tasks before it shipped.

`CONTEXT.md`'s **Build Cache** entry stops being Rust's alone, and
**descriptor** becomes a term of its own beside it, with its own _Avoid_ line —
what it is, that it is data rather than behaviour, that behaviour is a
capability it names, and that the built-ins and an installer's own are the one
grammar. The **Compile Server** entry says it comes up on the switch rather
than on a manifest at a Repo's root, and says what detection is still for.

The adoption doc gains the grammar, with the built-ins as its worked examples —
that being the point of shipping them in the grammar an installer writes. What
it has to show: an entry that overrides one variable of a built-in and leaves
the rest, a variable taken out with `null`, a language Verkstead has never heard
of, the two placeholders and when to reach for which, and what an entry that
does not load costs the person who wrote it.

Everywhere else that describes `config.yaml`'s shape describes the new one: the
development doc and the design doc both name the old key.

## Acceptance criteria

- [ ] `CONTEXT.md` carries the descriptor as a term of its own, and its Build
      Cache and Compile Server entries agree with what shipped.
- [ ] The adoption doc shows the grammar, a worked override, a `null`, a language
      of the installer's own and both placeholders, and says what an entry that
      does not load costs them.
- [ ] Every other place that describes `config.yaml`'s shape describes the new
      one.
