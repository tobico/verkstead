# 03. The grilling skill and the vocabulary

## What to build

The agent that writes the closing Set knows to say what the work wants, and
the glossary says what this stage made true.

- **The grilling skill's *The closing Set*** shows `wants` in its template
  and says when to write one: only where the work can be verified nowhere
  else — code under a platform `cfg`, a platform's own component (ConPTY,
  AppContainer, an installer, a bundle), or a suite that runs only on that
  operating system — the same rule a stage line's `on <os>` follows
  (ADR-0029, *Declaring what the work wants*). Naming a platform in prose
  wants nothing; shared code with a cross-platform suite wants nothing. It
  says what the human sees: the reason beside the device's tick.
- **The Guide** documents no `proposal` block (only a Response's
  `direction`), so it is left alone unless that has changed by now.
- **CONTEXT.md**: **Proposal** says it may carry `wants` and the chooser
  draws the ticks; **Permitted Device** says a tick can also come from the
  chooser. Each loses only the part of its *Being revised by ADR-0029*
  pointer this stage made true, keeping what later stages still owe.

## Acceptance criteria

- [ ] The template's YAML, with `wants`, is a Set the schema accepts (checked
      by a test if the skill's other examples are, by hand otherwise).
- [ ] The skill says when not to write `wants`, and a proposal with no
      platform wants nothing and draws as today.
- [ ] CONTEXT.md's two terms read as true after this stage, with no word from
      their *Avoid* lists.
