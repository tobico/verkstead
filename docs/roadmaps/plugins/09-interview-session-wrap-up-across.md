# 09. The interview, one session and the wrap-up across

## Goal

Three more built-in Activities cross the boundary stage 08 drew, and with them
every session a Develop, Review or Fix Merge Issues Conversation runs is
launched by a component: the **interview**, which is the grilling session and
ends in the Direction picked on its closing Set; **one session**, which is an
inline build, a `submitting` step or an `instruction` steer under a skill with
the Done-signal evidence it expects; and the **wrap-up**, one Activity with
declared watchers over the pull request — checks, comments, the review,
mergeability — that opens the pull request where none stands and ends when it
is merged, closed or given up on. Investigating rounds are wired as the
configuration of stage 08's follow-up rounds they were decided to be. The
sessions suite passes against the components, the Plugins page lists the three
under Built-in, and nothing on the workbench has moved.

## Decisions in force

- **[ADR-0030](../../adr/0030-plugins.md), *Activity*.** Built-in Activities
  are components, for the reason stage 08 gives. This stage crosses the three
  that carry the most server behaviour short of the runners, so that stage 11
  can write Develop as data over nothing but Activities.
- **The interview is one Activity whose outcomes are the Directions.** It
  launches the grilling session on `verkstead/grilling` under the Grilling
  role; the proposals it puts are honoured by the host as today
  (`settle_a_proposal`, `write_the_artifact`); and its outcome fires when the
  session's `verkstead done` is accepted with the picked Direction's artefact
  checked — the handoff at `/tmp/verkstead/handoff.md` for inline, the plan
  commit for a task list, the roadmap commit and pull request for a roadmap
  ([ADR-0008](../../adr/0008-pick-informs-artifacts-move.md)). Its outcomes
  are `direction:inline`, `direction:task-list` and `direction:roadmap`, plus
  `steered`. Breaking down and staging go on inside the same session, as the
  grilling skill says, so the interview is still one session: it does not
  launch a second for either.
- **A Process declares which Directions its interview offers** (Set 1765,
  Q27): the chooser draws those and no others, and one Direction is no chooser
  at all. That is the Process's declaration (stage 11) read by this Activity as
  an option, so the interview takes the offered list as a start fact and the
  host draws the chooser from it.
- **One session is the plain Activity**: a skill, a role, a prompt, an evidence
  to expect, and the outcome `done` — or `refused` where the Done signal never
  came and the session was stopped. The inline build under `verkstead/
  implementing` with the handoff, the `submitting` step, and an `instruction`
  steer are all this Activity with different arguments. Memos and previews
  stay permitted by the kind of building the Activity registers
  (`memos::Building`), which becomes an argument too.
- **Investigating rounds are follow-up rounds configured**, as stage 08
  decided: the skill `verkstead/investigating`, the evidence `Investigated`,
  the mark the same, and the ending checks that nothing was committed rather
  than reading the branch. No new Activity; the Investigate Process and the
  Investigating detour state (stage 11) name the follow-up Activity with those
  options. The Done signal accepted over uncommitted scratch here and nowhere
  else ([ADR-0020](../../adr/0020-a-conversation-has-a-process.md),
  *Investigate*) is an option of the evidence rather than a rule of the state.
- **The wrap-up is one Activity with declared watchers** (Set 1767, Q32).
  Several Activities, one per watcher, with a state running several at once was
  offered and turned down: a state runs one Activity, and the watchers run
  concurrently inside it the way `checks.rs`, `comments.rs`, `review.rs` and
  `merges.rs` already run beside each other. Fix Merge Issues is the wrap-up
  with the review and comments watchers off, which is what pre-settling them
  did (ADR-0020, *Fix Merge Issues*). Its options are the four watchers; its
  outcomes are `merged`, `closed`, `given-up` and `steered`.
- **The wrap-up opens the pull request where none stands**, by launching one
  session on `verkstead/submitting` — one Activity calling for another is not
  allowed, so the wrap-up launches that session itself with the evidence
  `Landed`, as the wrap-up's *no pull request* entry does today.
- **The watchers reach GitHub through the Forge host function**, which this
  stage leaves wrapping `github.rs`; stage 12 puts the Forge interface behind
  it and the watchers do not change. Where a Forge lacks a capability a
  watcher needs, the watcher is settled at once (ADR-0030, *Forge*) — this
  stage writes the settled-at-once path so stage 12 only has to feed it.
- **Each crossing leaves the suite green against the component.** The tests
  in `crates/server/tests` that drive a grilling through to a Direction, an
  inline build to its pull request, and a wrap-up through checks, comments and
  review are the acceptance tests; none is rewritten to suit the component.

## Proposed tasks (provisional)

1. **One session** — the Activity, as a component in the built-in plugin:
   skill, role, prompt, evidence, building kind; outcomes `done`, `refused`,
   `steered`. The inline build, `submitting` and `instruction` run under it.
   AC: an inline Develop builds to a pull request under the component; a steer
   with an instruction runs and returns; memos and previews are still refused
   by kind.
2. **The interview** — the grilling session, proposals, the Direction as
   outcome, the offered-Directions start fact and the chooser drawn from it.
   AC: a grilling ends in each of the three Directions with the right artefact
   checked and the outcome fired; a Process offering one Direction shows no
   chooser; the interview's own Sets still carry the Diff.
3. **Investigating as follow-up rounds configured** — the Investigate start
   path and the Investigating detour launch stage 08's Activity with the
   investigating options. AC: an Investigate Conversation runs its rounds and
   lands Done over uncommitted scratch; a steered Investigating returns to the
   state it came from with the scratch taken back out, as `homeward` does.
4. **The wrap-up** — one component with the four watchers as options, the
   `submitting` launch where no pull request stands, checks with their
   fix-attempt counts, comments dispatching `responding` and `addressing`, the
   review under the Review role, mergeable and landing; outcomes `merged`,
   `closed`, `given-up`. AC: the wrap-up tests pass; Fix Merge Issues runs
   with two watchers off and no review session ever launches; a watcher over
   an absent capability is settled at once.
5. **The runner emptied** — `runner.rs`'s drivers for these paths, `wrapping.rs`
   move-into-Wrapping on recording a pull request, `grillings.rs`,
   `handoffs.rs` and `investigations.rs` reduced to what the host functions
   still need. AC: no driver code for these paths remains outside the host
   functions and the components; the Plugins page lists *Interview*, *One
   session* and *Wrap-up* under Built-in.

## Re-verify at start

- Stage 08 landed: the `activity` world, the host functions and follow-up
  rounds as a component — and whether the world gained anything in the
  meantime that these three should use.
- `conversations.rs` `start_grilling` (near 1621) and `take_up` (near 3499)
  are still where Start chooses the first session; stage 11 replaces that
  choice with the Process's declared first state, so here they call the
  Activities by name.
- `wrapping.rs` still moves a Conversation into Wrapping on recording a pull
  request (`store/pull_requests.rs`); the wrap-up Activity's start must be that
  moment until stage 11.
- `checks.rs` still polls every 30 s and counts fix attempts per check name in
  `wrap_up.rs`'s `check_fix_attempts`; `comments.rs` still keys addressed
  comments by GitHub node id — stage 12 will make that id the Forge's.
- `investigations.rs` `homeward` is still how a steered Investigating finds its
  way back, and the steer still records its source state.
- The grilling skill still says breaking down and staging continue inside the
  grilling session, so the interview launches one session only.
