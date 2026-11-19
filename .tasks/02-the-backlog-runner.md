# 02. The backlog runner for a task list

## What to build

The backlog runner as a component in the built-in Activities plugin
(`plugins/activities/`), declared beside follow-up rounds with its outcomes and
its skill (`verkstead/next-task`), and run where `runner.rs` runs the backlog
today for a **task-list Direction**: when the grilling's breakdown lands
(`build_the_split_out`, `follow_breakdown`) and when a stopped run is taken up
again (`backlog_again` via `implementing_again`).

What the component does, reading the Repo through task 01:

- the next step is the lowest-numbered unticked entry of `.tasks/TODO.md` at
  HEAD, in the list's own order; with every box ticked it is the finish;
- a task launches a session on `verkstead/next-task` expecting
  `landed(ticked(n))`, ending with its work;
- the finish launches the same skill expecting `landed(gone(".tasks/TODO.md"))`,
  ending on a pull request;
- an unticked entry naming no `NN-<slug>.md` stops the Conversation with the
  sentence `nothing_to_work_from` says today;
- it writes the session's prompt itself, in the words `skills::next_task`
  writes now (move them, as follow-up rounds moved its builder);
- it keeps nothing: started again, it reads the boxes again.

**Outcomes** (Set 1839, Q6): `finished`, once the finish's Done signal is
accepted with its pull request open, and `steered`. There is no `abandoned`: a
step whose session is gone or stalled is a `host::stop`, as follow-up rounds
does, and Resume starts the component again. On `finished` the host does what
`to_a_pull_request` does now: records the pull request and starts the wrap-up.
An empty backlog on Resume (`nothing_left`, which asks GitHub) stays host-side
until stage 12's Forge.

**Memos and previews need no new argument.** The host derives the building
kind (`memos::Building`) from the evidence a launch expects: `ticked(n)` is
`Task(n)`, `gone(TODO.md)` is `Finish`, `arrived(TODO.md)` is `Stage` (task 03).
Stage 09 may add a `building` argument for its one-session Activity; that
does not conflict with this.

A stage's backlog (planning, the join, placed tasks) is task 03. Until then a
stage's backlog keeps running through `runner.rs` as it does now.

## Acceptance criteria

- [ ] A task-list Develop runs from its breakdown to its pull request under the
      component, and the Plugins page lists *Backlog runner* under Built-in
- [ ] A server restarted mid-backlog relaunches at the right box with nothing
      stored for the Activity
- [ ] The finish still refuses a task session that also deleted the backlog, as
      `done.rs` does today
- [ ] An unticked entry with no task file stops the run naming the entry
- [ ] The sessions suite's existing backlog tests pass without being rewritten
