# 03. Wrap-up

## Goal

The full unattended pipeline: features finish themselves (push + draft PR),
every PR gets a wrap-up phase (agent self-review, CI monitoring with
auto-fix, PR-comment dispatch), staged roadmaps execute end to end with
stages stacking on unmerged predecessors — and Verkstead replaces roadrunner
and the tobico-scripts wrappers for daily work. Adoption happens when this
stage lands.

## Decisions in force

From [docs/design/verkstead.md](../../design/verkstead.md):

- **Finish runs unattended** per the repo's review process
  (`docs/agents/git-workflow.md` in the target repo): push + draft PR, or
  `gh stack submit --auto` on stacked branches. Merging stays a human act.
- **Wrap-up phase per PR** (the Q18b addition): a fresh-context session
  reviews the opened PR and raises a question set about any issues found;
  Verkstead monitors the CI run and dispatches fix sessions on failure —
  **two attempts, then a blocking ask**. New PR comments are detected by
  polling and auto-dispatch an addressing session. The next stage starts only
  after wrap-up completes.
- **Commit feedback consolidates in wrap-up** — this is where review-later
  becomes real; there are no earlier review states to migrate.
- **Stages always stack** on the unmerged predecessor branch via `gh stack`.
- **Verkstead reaches GitHub through host `gh`** — CI status, PR commit list,
  comments — reusing existing auth; no token store, no GitHub App. Agents
  keep using `gh` inside the sandbox for push/PR.
- **PR timeline event**: pinned summary (name + id); details pane shows the
  fetched commit list and comments.
- **Stage list event**: `docs/roadmaps/` in the worktree parsed into the
  pinned stage-list event; stages auto-continue (the next stage's
  task-breakdown quiz blocks on questions naturally).
- **Roadmap direction** from stage 02's chooser becomes executable: the
  bundled fork of to-roadmap/next-stage writes and consumes the same repo
  file formats.

## Proposed tasks (provisional)

1. **Unattended finish** — bundled fork's finish sequence runs without a
   gate; PR event appears pinned with commit list + comments fetched via
   host gh.
2. **CI monitoring** — poll the PR's checks; failure dispatches a fix
   session; two failed fixes raise a blocking ask; success advances wrap-up.
3. **Self-review loop** — fresh-context PR review session on PR open; its
   findings arrive as a question set; approved fixes dispatch sessions.
4. **PR-comment dispatch** — poll for new comments during Wrapping;
   auto-dispatch an addressing session per batch.
5. **Staged roadmaps** — roadmap direction writes `docs/roadmaps/` via the
   bundled fork; stage-list pinned event; auto-continue into the next stage
   after wrap-up, stacking with `gh stack init`/`add`.
6. **Retirement pass** — document the switch-over; verify a real repo's
   workflow runs end to end in Verkstead; mark roadrunner and the wrappers
   deprecated in their repos.

## Re-verify at start

- Assumes stages 01–02 landed (sessions, task runner, commit events,
  direction chooser).
- Assumes `gh` and the `gh-stack` extension still behave as
  git-workflow.md describes (`submit --auto`, `sync`); verify against
  installed versions.
- Assumes CI status is reachable via `gh pr checks` / `gh run` polling —
  confirm the exact commands against the gh version on the host.
- Assumes the wrap-up self-review wants a distinct bundled skill (a
  review-prompt fork) — decide its shape at to-tasks time.
