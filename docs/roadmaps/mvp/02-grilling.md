# 02. Grilling

## Goal

A grilling session runs entirely from the GUI: press *start grilling* on a
conversation and the agent runs in a bwrap sandbox on a fresh worktree, its
transcript streaming into the timeline, its question sets answerable in the
workbench and on the phone. Blocking asks only; nothing after grilling yet.

Split out of the original stage 01 on 2026-08-20 — this is its second half,
stacking on [01. Workbench](01-workbench.md), which supplies the repos,
conversations and profiles this stage executes against.

## Decisions in force

All from [docs/design/verkstead.md](../../design/verkstead.md) unless marked
otherwise; the ones this stage builds on:

- **Sandbox: bwrap minimum surface** (rw worktree + common `.git` + profile
  pair; ro nix/system/gitconfig/gh; tmpfs rest; per-repo extras from global +
  per-repo sandbox config; full network; Nix dev-shell autodetection). Drops
  the blanket `~/src` bind that `tobico-scripts/bin/sandbox` has today.
- **The NixOS module has to learn to run the orchestrator** *(decided
  2026-08-20 at to-tasks time)*. `nix/module.nix` today sets
  `RestrictNamespaces = true`, `PrivateUsers = true`, `ProtectHome = true`,
  `CapabilityBoundingSet = [""]` and `SystemCallFilter = ~@privileged`, which
  between them stop bwrap unsharing anything and hide the watched paths
  entirely. Relax each to what bwrap actually needs, open the watched paths,
  and extend `nix/vm-test.nix` to prove a sandbox starts under the unit.
  *Why now rather than later:* the VM test boots the module, and a unit that
  cannot spawn a sandbox is a unit testing the wrong product.
- **Question delivery: conversation-scoped `VERKSTEAD_SERVER`** injected into
  the sandbox; the bundled CLI attributes sets explicitly. Sessions idle
  while blocked (ADR 0001 in tobico-skills: interactive sessions, never
  headless `-p`).
- **PTY capture** ported from roadrunner's `session.ts` (claude under
  `script`, live mirror + full transcript stored; timeline summary = line
  count + latest statement).
- **The bundled skill fork must carry the ask instruction itself** *(found
  2026-08-20 while re-grounding)*. `~/src/tobico-skills/skills/grilling/`
  is twelve lines of generic "interview me relentlessly" and never mentions
  askance or question sets — what actually makes an agent ask is the host's
  global `CLAUDE.md`, which the sandbox will not have. Either the bundled
  fork says it, or the profile's `~/.claude/CLAUDE.md` does.
- **Skills are bundled**: Verkstead ships its own adapted fork of the
  tobico-skills set and installs it into each sandbox; `~/src/tobico-skills`
  is no longer bound in.
- **UI:** output and question sets summarized in the timeline with details on
  click. Question sets answerable in the workbench and on the phone alike.
- **`/pending` and `/archive` retire here** *(decided 2026-08-20 at to-tasks
  time)*. Stage 01 kept them so the phone never stopped answering; once
  question sets arrive in the timeline they are reached through their
  conversation, and the transitional routes go.

## Proposed tasks (provisional)

1. **Worktree + sandbox launcher** — orchestrator creates branch + worktree
   under the state dir; bwrap spawn with the minimum surface; the NixOS
   module relaxed to permit it; teardown on archive.
   - a session inside sees only the decided surface (verified by a probe)
   - the VM test starts a sandbox under the packaged unit
2. **Grilling session + transcript events** — *start grilling* launches the
   grilling profile's claude with the bundled grilling skill; PTY capture;
   live-updating output event; full transcript in details.
3. **Question sets in the timeline** — conversation-scoped
   `VERKSTEAD_SERVER`; sets appear as timeline events summarized as a
   #/question/answer table; answerable in workbench and phone; blocking
   semantics end-to-end; `/pending` and `/archive` retired.
   - a full grill loop (brief → questions → answers → more questions) works
     from the GUI alone

## Re-verify at start

- Assumes stage 01 landed: the rename, watched paths, repo registration,
  conversations with briefs, and agent profiles.
- Assumes `bwrap`, `nix`, `script` (util-linux), `claude` and `gh` exist on
  the host; assumes NixOS.
- Assumes roadrunner's `session.ts`/`done-signal.ts` in
  `~/src/tobico-skills/src/` are still the reference implementations to port.
- Assumes the grilling skill's current shape in
  `~/src/tobico-skills/skills/grilling/` as the basis for the bundled fork —
  and note it is a twelve-line prompt, so the fork is mostly new writing.
- Assumes the CLI's environment variable is `VERKSTEAD_SERVER` after stage
  01's rename.
