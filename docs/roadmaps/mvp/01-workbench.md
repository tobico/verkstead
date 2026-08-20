# 01. Workbench

## Goal

The workbench exists and holds everything a grilling session will need, with
nothing executing yet: the repo is Verkstead by name throughout, repos are
registered from inside the watched paths, and a conversation carries a brief,
an editable branch name and its two agent profiles — ready to grill, with the
*start grilling* button the only thing missing.

Split out of the original stage 01 on 2026-08-20, when re-grounding found that
stage combining a whole-repo rename with a process supervisor. Its second half
is [02. Grilling](02-grilling.md), which stacks on this one.

## Decisions in force

All from [docs/design/verkstead.md](../../design/verkstead.md) unless marked
otherwise; the ones this stage builds on:

- **Clone, not rewrite.** The askance server, store, SPA, rendering pipeline,
  SSE nudges and push stack are kept and extended. The phone answering flow
  must keep working after every task in this stage.
- **Rename first.** Crates, binary, branding, nix module, docs become
  Verkstead; CONTEXT.md starts the Verkstead glossary (conversation, brief,
  timeline, event, watched paths, agent profile, grilling/implementation
  class, blocking/deferred ask). *Why first:* every later commit is otherwise
  written against a name that's about to change under it.
- **The agent-facing surface renames too** *(decided 2026-08-20 at to-tasks
  time)*: `verkstead ask`, `VERKSTEAD_SERVER`, `VERKSTEAD_DATABASE`,
  `VERKSTEAD_LISTEN`. The design document writes `ASKANCE_SERVER` because it
  predates the rename being firm. *Why:* the real askance stays installed on
  the host for daily work, and one name answering to two different binaries
  depending on which sandbox you are in is a trap.
- **Watched paths are a security boundary**, configured in the environment at
  installation — not a convenience scan. Repo registration and every
  filesystem operation validate against them.
- **Conversation = repo + base commit + brief + one branch + one worktree.**
  Base commit defaults to default-branch tip at grill start, overridable.
  Branch name prefilled randomly, editable while drafting. Worktrees live
  under Verkstead's state dir, kept until the conversation is archived. The
  worktree itself arrives in stage 02; this stage records the intent.
- **Agent profiles are minimal** (name, claude dir/config pair, default
  model, agent-type discriminator), bind-mounted over `~/.claude` /
  `~/.claude.json` — the account-separation trick from
  tobico-scripts/`work-sandbox`. A conversation fixes a grilling profile and
  an implementation profile before grilling starts.
- **UI: responsive 3-pane** — conversations sidebar (manual ordering can wait;
  attention markers can wait), timeline, details pane. Brief inline.
- **The workbench takes `/`** *(decided 2026-08-20 at to-tasks time)*; the
  pending list moves to `/pending`, with `/archive` and `/sets/:id`
  unchanged, so the phone keeps answering throughout. `/pending` and
  `/archive` are **transitional**: the workbench replaces them, and question
  sets are reached through their conversation once stage 02 lands them in the
  timeline. Retiring them is stage 02's business, not this one's.

## Proposed tasks (provisional)

1. **Rename to Verkstead** — crates, binary, CLI verb and environment
   variables, SPA branding, nix module, CI, docs; Verkstead glossary begun in
   CONTEXT.md.
   - `nix flake check` and CI green under the new name
   - `verkstead serve` serves the (unchanged) askance viewer, and
     `verkstead ask` still puts a set to the human
   - git-workflow.md notes updated for this repo (origin, private)
2. **Watched paths + repo registration** — install-time config; registry API;
   GUI list + add-by-path; refusal outside watched paths.
   - a repo outside watched paths cannot be registered or touched
   - registered repos survive restart (SQLite)
3. **Conversations + briefs** — the 3-pane workbench shell at `/`;
   create/select in the sidebar; markdown brief editor; random branch-name
   prefill, editable; base-commit capture rule.
   - a conversation renders as a timeline with the brief as first inline event
   - brief edits persist; branch name customizable until grill start
   - the phone's pending list still answers, at `/pending`
4. **Agent profiles** — CRUD in GUI; per-conversation grilling/implementation
   selection; profile validation (dir pair exists inside watched-path rules).
   - two profiles selectable on a conversation before grilling

## Re-verify at start

- Assumes the askance tree as of commit `6f32b11` (v0.1.0) — check whether
  upstream askance moved and whether any divergence matters.
- Assumes NixOS, and `nix` and `gh` on the host.
- Assumes the store still has no migration machinery
  (`CREATE TABLE IF NOT EXISTS` against a STRICT `question_sets`), so new
  tables go in by editing `apply_schema` rather than by migrating — which the
  fresh-database decision permits.
