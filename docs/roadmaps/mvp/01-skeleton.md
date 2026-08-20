# 01. Skeleton

## Goal

A grilling session can be run entirely from the GUI: register a repo, write a
brief in a conversation, pick agent profiles, press *start grilling*, and
answer the resulting question sets in a timeline — with the agent running in
a bwrap sandbox on a fresh worktree, and its transcript streaming into the
conversation. Blocking asks only; nothing after grilling yet.

## Decisions in force

All from [docs/design/verkstead.md](../../design/verkstead.md); the ones this
stage builds on:

- **Clone, not rewrite.** The askance server, store, SPA, rendering pipeline,
  SSE nudges, and push stack are kept and extended. The phone answering flow
  must keep working after every task in this stage.
- **Rename first.** Crates, binary, branding, nix module, docs become
  Verkstead; CONTEXT.md starts the Verkstead glossary (conversation, brief,
  timeline, event, watched paths, agent profile, grilling/implementation
  class, blocking/deferred ask). *Why first:* every later commit is otherwise
  written against a name that's about to change under it.
- **Watched paths are a security boundary**, configured in the environment at
  installation — not a convenience scan. Repo registration and every
  filesystem operation validate against them.
- **Conversation = repo + base commit + brief + one branch + one worktree.**
  Base commit defaults to default-branch tip at grill start, overridable.
  Branch name prefilled randomly, editable while drafting. Worktrees live
  under Verkstead's state dir, kept until the conversation is archived.
- **Agent profiles are minimal** (name, claude dir/config pair, default
  model, agent-type discriminator), bind-mounted over `~/.claude` /
  `~/.claude.json` — the account-separation trick from
  tobico-scripts/`work-sandbox`. A conversation fixes a grilling profile and
  an implementation profile before grilling starts.
- **Sandbox: bwrap minimum surface** (rw worktree + common `.git` + profile
  pair; ro nix/system/gitconfig/gh; tmpfs rest; per-repo extras from global +
  per-repo sandbox config; full network; Nix dev-shell autodetection). Drops
  the blanket `~/src` bind.
- **Question delivery: conversation-scoped `ASKANCE_SERVER`** injected into
  the sandbox; the bundled CLI attributes sets explicitly. Sessions idle
  while blocked (ADR 0001 in tobico-skills: interactive sessions, never
  headless `-p`).
- **PTY capture** ported from roadrunner's `session.ts` (claude under
  `script`, live mirror + full transcript stored; timeline summary = line
  count + latest statement).
- **UI: responsive 3-pane** — conversations sidebar (manual ordering can wait;
  attention markers can wait), timeline, details pane. Brief inline; output
  and question sets summarized with details on click.

## Proposed tasks (provisional)

1. **Rename to Verkstead** — crates, binary verbs, SPA branding, nix module,
   CI, docs; Verkstead glossary begun in CONTEXT.md.
   - `nix flake check` and CI green under the new name
   - `verkstead serve` serves the (unchanged) askance viewer
   - git-workflow.md notes updated for this repo (origin, private)
2. **Watched paths + repo registration** — install-time config; registry API;
   GUI list + add-by-path; refusal outside watched paths.
   - a repo outside watched paths cannot be registered or touched
   - registered repos survive restart (SQLite)
3. **Conversations + briefs** — create/select in sidebar; markdown brief
   editor; random branch-name prefill, editable; base-commit capture rule.
   - a conversation renders as timeline with the brief as first inline event
   - brief edits persist; branch name customizable until grill start
4. **Agent profiles** — CRUD in GUI; per-conversation grilling/implementation
   selection; profile validation (dir pair exists inside watched-path rules).
   - two profiles selectable on a conversation before grilling
5. **Worktree + sandbox launcher** — orchestrator creates branch + worktree
   under the state dir; bwrap spawn with the minimum surface; teardown on
   archive.
   - a session inside sees only the decided surface (verified by a probe)
6. **Grilling session + transcript events** — *start grilling* launches the
   grilling profile's claude with the bundled grilling skill; PTY capture;
   live-updating output event; full transcript in details.
7. **Question sets in the timeline** — conversation-scoped `ASKANCE_SERVER`;
   sets appear as timeline events summarized as a #/question/answer table;
   answerable in workbench and phone; blocking semantics end-to-end.
   - a full grill loop (brief → questions → answers → more questions) works
     from the GUI alone

## Re-verify at start

- Assumes the askance tree as of commit `6f32b11` (v0.1.0) — check whether
  upstream askance moved and whether any divergence matters.
- Assumes `bwrap`, `nix`, `script` (util-linux), `claude`, and `gh` exist on
  the host; assumes NixOS.
- Assumes roadrunner's `session.ts`/`done-signal.ts` in
  `~/src/tobico-skills/src/` are still the reference implementations to port.
- Assumes the grilling skill's current shape in
  `~/src/tobico-skills/skills/grilling/` as the basis for the bundled fork.
- Assumes `ASKANCE_SERVER` env var is still how the CLI finds the server.
