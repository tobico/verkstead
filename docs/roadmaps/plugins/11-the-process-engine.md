# 11. The Process engine

## Goal

A plugin may provide a **Process**: declared states over Activities, with
transitions keyed on the outcomes those Activities end in, and Verkstead runs a
Conversation by reading its Process's manifest rather than by matching on a
shared enum. The five built-in Processes — Develop, Investigate, Review, Tinker
and Fix Merge Issues — are written as data in bundled plugins, listed under
Built-in on the Plugins page, and every Conversation already in the database
reads its state against the plugin its Process names with nothing migrated.
The shared `Lifecycle` is gone, the viewer draws state words, picker rows,
descriptions and steer rows from what the manifests declare, and the human's
design Process — rounds with pictures under an attached MCP server, an export
session, the wrap-up — can be written as a plugin naming nothing but existing
Activities and a Skill pack of its own.

## Decisions in force

- **[ADR-0030](../../adr/0030-plugins.md), *Process*.** States over Activities
  (Set 1765, Q23, option 1). Each state runs one of Verkstead's Activities
  with declared options; transitions map each outcome that Activity declares to
  a state. States that are each a bare session, with the composite machinery
  kept in a hard-coded Develop, was offered and turned down, and so was a fully
  general machine with guards and actions as wasm exports — the first cannot
  write Develop and the second makes every Process a component. Activities
  being plugins themselves (stages 08 to 10) is what lets this be data.
- **Draft, Done and Closed are Verkstead's** (Q24): the three states nothing
  returns to, each with a way in of its own, unnamed in the manifest. The
  plugin declares the states between; Start enters the first declared state.
- **A transition for every outcome.** `verkstead plugin check` and the loader
  refuse a Process whose declared Activity has an outcome the manifest leaves
  unmapped. `steered` is an outcome every Activity has, and a Process maps it
  by marking which states take a steer and with what brief rather than by a
  transition row — see Steer below.
- **Roles are the plugin's** (Q26): declared by name and label, one marked as
  the **building role** that the Done signal, memos and previews assume. The
  three names Verkstead knows — `grilling`, `implementation`, `review` — keep
  their per-Repo memory and platform defaults; a name nobody has used before
  gets a memory of its own the first time it is picked. The Agent control's
  shape follows the role count, one role a flat dropdown and more a panel, so
  [ADR-0020](../../adr/0020-a-conversation-has-a-process.md)'s table becomes
  the manifests'. Fixing the three and having a plugin pick among them was
  turned down.
- **Directions are offered by the Process** (Q27): the manifest maps the
  Directions its interview offers to states, the chooser draws only those, and
  one Direction is no chooser. Stage 09's interview takes the offered list as a
  start fact; this stage is where a Process supplies it. ADR-0008's rule that
  the pick decides the artefact is unchanged.
- **Every steer target is a declared state, Investigating included** (Q28,
  option 2 — against the recommendation). A universal detour owned by Verkstead
  was offered and turned down: Develop declares its own `investigating` state,
  marked as a **detour that returns to the state it was steered from**, and
  Investigate is the Process that is nothing but that detour entered from a
  Draft. A state marked to take a steer says the brief it takes; the Steer form
  draws its rows from those marks. A steer still records its source state, as
  ADR-0020 decided, and a detour reads it back when its Activity ends.
- **Words come from the manifest** (Q29): the Process's name and description on
  the picker, each state's word on the Conversation card, the steer rows. The
  viewer's hand-kept tables — `web/src/workbench/states.ts`, `processes.ts`,
  `Steer.tsx`'s `TARGETS`, the mirrored `LANDED`, `takes_a_target` and
  `narrows_the_wrap_up` lists — go, replaced by data the server sends. Spelling
  rules in `docs/copy.md` hold for the built-ins' words; a third-party word is
  the plugin's own.
- **Whether a Process takes a Target**, and whether it narrows the wrap-up,
  are manifest facts: the Target field is drawn where the manifest says, and
  Fix Merge Issues is the wrap-up with the review and comments watchers off
  rather than a special path (stage 09).
- **The Nothing-else box and the proposal chooser follow the running
  Activity**, not the state word: a Set from follow-up rounds carries the box,
  a Set from the interview honours a `proposal`, whatever the state is called.
  Today both are gated on `Lifecycle` (`Directing::NotGrilling`, the `Ending`
  on FollowUp and Investigating rounds).
- **Nothing migrates.** The `conversations.state` column already holds a word;
  the built-in plugins name their states with the words already stored
  (`grilling`, `implementing`, `wrapping`, `follow_up`, `investigating`), and
  a Conversation from before Processes still reads Develop, as the `processes`
  side table decided. The `Lifecycle` enum in `store/conversations.rs`, its
  render mirror and its ts-rs export go; a state is a word read against a
  manifest, and an unknown word — a plugin removed under a Conversation —
  draws as the word with its trouble, which Remove being refused while
  anything stands on the plugin should make rare.
- **Disabling a Process plugin** takes its row off the picker; Conversations on
  it run to the end, as for Agents (Set 1764, Q20, applied by me in Set 1765).

## Proposed tasks (provisional)

1. **The `process` provides-block** — states with word, Activity, options,
   steer mark and detour mark; transitions; offered Directions; roles with the
   building role; Target and name and description; the check refusing an
   unmapped outcome. AC: `verkstead plugin check` accepts a complete Process and
   names the outcome a broken one leaves unmapped; the loader does the same.
2. **The engine** — Start enters the first state, runs its Activity with the
   Conversation's facts, maps the outcome, enters the next; Done and Closed
   are reached by transition or by the human's Close; restart re-enters the
   current state's Activity. AC: a fixture Process of two states over the
   one-session Activity runs a Conversation Draft to Done; a restart carries
   on.
3. **The five as data** — Develop (interview, three Directions, one session,
   backlog runner, stage runner, wrap-up, the `investigating` detour),
   Investigate, Review, Tinker, Fix Merge Issues, each a bundled plugin. AC:
   the whole sessions suite passes with the five read from manifests; a
   Conversation stored before this stage reads its state and runs on.
4. **Lifecycle goes** — the enum, its ~33 match sites, the render mirror and
   the generated type replaced by the state word and the manifest; `resume.rs`,
   `drivers.rs`, `stalls.rs`, `steering.rs`, `boundaries.rs`, `reports.rs`,
   `transfers.rs` read the manifest. AC: no `Lifecycle` in the tree; a
   Transfer carries a Conversation of a third-party Process.
5. **Steer from the manifest** — the Steer form's rows from the steer marks,
   the brief each takes, detours returning where they came from, roles per
   target from the manifest. AC: a steer into Develop's `investigating` returns
   to the state it left; a Process with no steerable state draws no Steer rows
   but Close.
6. **The viewer from the manifest** — picker rows, descriptions, state words,
   the Agent control's shape by role count, the Target field, the Nothing-else
   box and chooser by Activity. AC: the hand-kept tables are gone; a
   third-party Process installed appears on the picker with its words.
7. **The design Process as the acceptance test** — a fixture plugin: follow-up
   rounds under its own skill with an MCP server attached, one session to
   export, the wrap-up. AC: it loads, runs in the suite, and needs no code.

## Re-verify at start

- Stages 08, 09 and 10 landed: every Activity is a component with declared
  outcomes, and the list of outcomes each declares is what the five manifests
  map — read them off the built-in plugins rather than this brief.
- `store/conversations.rs` still holds `Lifecycle` near line 69 with eight
  states and the `processes` side table near 1441; `ui.rs` still maps both to
  render enums near 6578.
- `steering.rs` `roles(target, process)` near 1496 and the target-to-state
  table near 2697 are still where steer rows are decided.
- The Nothing-else `Ending` and `Directing::NotGrilling` are still gated on the
  state in the store (`own_sets.rs`, `conversations.rs` near 4849).
- Stage 07's Skill packs are attachable, which the design Process fixture
  relies on for its own skill.
- Whether any stage since 01 added a manifest grammar feature — options,
  per-platform overrides — that a Process block should reuse rather than
  invent.
