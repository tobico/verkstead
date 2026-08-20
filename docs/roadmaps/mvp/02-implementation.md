# 02. Implementation

## Goal

A conversation goes from finished grilling to implemented work without the
CLI: the agent proposes wrap-up, the human picks a direction (inline or task
list) in the GUI, and sessions execute unattended — committing freely, one
fresh session per task — with commits appearing as reviewable diff events in
the timeline.

## Decisions in force

From [docs/design/verkstead.md](../../design/verkstead.md):

- **Grilling ends by agent proposal**: a final question set moves the
  conversation to Direction. No GUI wrap-up button.
- **Direction: agent recommends, human decides.** The recommendation (inline /
  task list / roadmap) comes with rationale. Roadmap execution itself is
  stage 03; here the choice UI plus inline and task-list paths land.
- **Inline = fresh session under the implementation profile, primed with a
  handoff document the grilling session writes.** *Why:* grilling and
  implementation run under different fixed profiles (fable vs opus today), so
  the grilling session cannot simply continue.
- **No commit gates, full auto-advance within a feature**: agent commits on
  its own; fresh session per task; task files deleted on completion
  (done-signal = task file gone *and* committed, ported from roadrunner's
  `done-signal.ts` with `--no-optional-locks` discipline).
- **Repo files stay the source of truth**: `.tasks/TODO.md` and `NN-*.md` in
  the worktree, written by the bundled skill fork; Verkstead parses them into
  the pinned task-list event. The bundled fork removes next-task's approval,
  context-clear, and finish gates.
- **Commit events** summarize +/− and changed-line counts; the details pane
  shows the server-rendered diff (askance's `render/diff.rs` — folds, syntect
  highlighting — fed per-commit rather than working-tree).
- **No per-commit review states.** Commits are viewable; feedback consolidates
  in wrap-up (stage 03).
- **Interruptions GUI-native**: crash/hang → timeline event with retry /
  take-over-manually / abort, porting roadrunner's remedies and evidence
  gathering (git status + session tail).
- **Blocked sessions idle**; the *blocked on you* badge shows on the
  conversation.

## Proposed tasks (provisional)

1. **Wrap-up proposal + Direction state** — grilling skill fork emits a
   marked final set; answering it transitions the conversation; direction
   chooser in the GUI shows the agent's recommendation.
2. **Handoff document + inline execution** — grilling session writes the
   handoff; *implement inline* launches an implementation-profile session
   primed with it; commits land unattended.
3. **Task-list path** — bundled `/to-tasks` fork runs in-conversation (its
   breakdown quiz arrives as ordinary question sets); `.tasks/` parsed into
   the pinned task-list event.
4. **Auto-advancing task runner** — port next-step + done-signal; fresh
   session per task; task list event updates as tasks complete.
5. **Commit timeline events** — commit detection in the worktree; +/− summary;
   per-commit diff rendering in the details pane.
6. **Interruption remedies** — detection (exit, hang past grace), evidence
   capture, retry / manual / abort actions from the timeline.

## Re-verify at start

- Assumes stage 01 landed: conversations, profiles, sandbox launcher,
  question-set plumbing, transcript events.
- Assumes the bundled skill fork exists from stage 01 (grilling) and needs
  to-tasks/next-task added — check what shape the fork took.
- Assumes roadrunner's next-step/done-signal semantics are still the ones to
  port (task file regex, stage entry regex, quiet-period grace).
- Assumes `render/diff.rs` can take `git show`-style per-commit input without
  structural change — verify against its parser.
