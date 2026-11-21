# 01. The `process` provides-block

## What to build

Give the `process` Category its grammar: a `provides:` entry a manifest reads,
`verkstead plugin check` checks and the loader refuses or holds in trouble —
written up in `docs/plugins.md` beside the `activity` grammar, and read the way
the `activity` and `skills` entries already are (refused at the first wrong
field, naming it; an unknown key refused rather than skipped; per-platform
overrides replacing whole keys as for every other entry).

A Process declares:

- `id`, `name` and `description` — what the picker draws and what the
  `processes` table names it by (see task 02 for how it is stored).
- `target` — whether the Process takes a Target, and `narrows-the-wrap-up`
  (Fix Merge Issues' fact; task 03 decides how the wrap-up reads it).
- `roles` — each by name and label, exactly one marked as the **building
  role**. Read and checked here; task 06 puts them behind the Agent control.
- `directions` — the Directions its interview offers, each mapped to a state.
- `states` — in order; the first is what Start enters. Each has its id (the
  word the `conversations.state` column holds), its word on screen, the
  Activity it runs — or, for a state the interview's Directions lead to, one
  Activity per Direction (`activity: {inline: one-session, task-list:
  backlog-runner, roadmap: stage-runner}`, the stored Direction picking; Set
  1879, Q8) — and that Activity's options, an optional **steer mark**
  (takes a steer, and which of Verkstead's brief kinds — `round`,
  `instruction`, `follow-up`, `question`, `none` — it takes; Set 1877, Q5),
  and an optional **detour** mark (returns to the state it was steered from).
- `transitions` — every Outcome of each state's Activity mapped to a declared
  state or to one of the targets Verkstead owns: `done`, `closed`, and
  `stopped`, which leaves the Conversation **Stopped** where it stands with
  the Activity's stop Notice (Set 1877, Q3; Set 1879, Q7). Draft is never a target. `steered`
  is never mapped by a row: the steer marks are its mapping.

Rough shape, for the author's reference (the grammar written in
`docs/plugins.md` is what is binding):

```yaml
- category: process
  id: tinker
  name: Tinker
  description: …
  target: false
  roles:
    - { name: implementation, label: Implementation, building: true }
  states:
    - id: follow_up
      word: Following up
      activity: follow-up-rounds
      options: { skill: verkstead/following-up }
      steer: { brief: follow-up }
    - id: wrapping
      word: Wrapping up
      activity: wrap-up
  transitions:
    follow_up: { committed: wrapping, nothing: done }
    wrapping: { settled: done, merged: done, closed: stopped, given-up: stopped }
```

**What a Process names, it declares** (Set 1771, Q1). Each state's Activity,
and each skill named in its options, is the Process's own plugin's or one of
its declared dependencies'. A skill a Process names in a state's options is
checked against the Process's plugin and its dependencies, not against the
Activity's own `skills:` list — that list is the Activity's defaults, and the
host lets a session launch under a skill the Process named as well as under
those.

`verkstead plugin check` and the loader refuse: an Outcome of a state's
Activity — of any of them, for a per-Direction state — left unmapped (naming the state and the Outcome), a transition from or
to a state not declared, an Activity or skill from a plugin that is not a
declared dependency (naming it), no building role or two, a Direction mapped to
no state. A Process whose Activity's plugin is missing, disabled or outdated
loads **in trouble**, its row on the Plugins page saying which dependency is
wanted, and is not offered — a picker listing comes in task 02, so here it is
the loader's record and the Plugins page that show it.

The six built-in Activities all sit in one always-on *Activities* plugin, and
the *Verkstead* pack plugin has no tick either, so the trouble path is proven
with a third-party fixture Process depending on a third-party fixture plugin
that is disabled or removed.

## Acceptance criteria

- [ ] `verkstead plugin check` accepts a complete Process and, for one with an
      Outcome unmapped, names the state and the Outcome
- [ ] A Process naming an Activity or a skill from a plugin it does not declare
      as a dependency is refused naming it, by the check and by the loader alike
- [ ] A fixture Process whose dependency is disabled shows its trouble on the
      Plugins page, saying which dependency is wanted, and comes back when the
      dependency is ticked again
- [ ] `docs/plugins.md` documents the `process` grammar, and its example
      passes `verkstead plugin check`
