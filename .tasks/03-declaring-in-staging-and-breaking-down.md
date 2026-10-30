# 03. Declaring, in staging and breaking-down

## What to build

The two planning skills say **when** a line declares `on linux`, `on macos` or
`on windows` (ADR-0029, *Declaring what the work wants*): **only where the work
can be verified nowhere else** — code under a platform `cfg`, a platform's own
component (ConPTY, AppContainer, an installer, a bundle), or a suite that only
runs on that operating system. Naming a platform in prose declares nothing, and
a change to shared code with a cross-platform suite declares nothing.

- **Staging** (`crates/server/skills/staging/SKILL.md`) already describes the
  `on <os>` tail on a stage line; it gains the criterion beside it, and its
  roadmap template shows one declared line.
- **Breaking-down** (`crates/server/skills/breaking-down/SKILL.md`) gains the
  same criterion for a task line in `.tasks/TODO.md`, written after the link as
  `— on windows`, and its task-list template shows one declared line. A task
  line declares nothing else — the order is the dependency. **Nothing reads a
  task line's platform yet** (stage 03 of this roadmap places by it and refuses
  a fourth word): say it is recorded rather than acted on, the way staging
  words a stage line's platform today. The checklist reader keeps a tail and
  ignores it, so the line is harmless until then.

Edit the source skills under `crates/server/skills/`; keep any skill-content
tests in the server suite passing, and add phrase checks where the suite already
checks these skills' words.

## Acceptance criteria

- [ ] Both skills say a line declares a platform only for a platform `cfg`, a
      platform's own component, or a suite that runs only there, and that prose
      or shared code with a cross-platform suite declares nothing.
- [ ] Both templates show a declared line.
- [ ] Breaking-down says a task line's platform is recorded and not yet acted
      on.
