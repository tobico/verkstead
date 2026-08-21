# Adoption

The switch-over: what Verkstead replaces, and how a day's work runs through it.
Working *on* Verkstead is [development.md](development.md); this is working
*with* it.

Until this point the loop was three tools kept in step by hand — askance for
the questions, `tobico-skills/roadrunner` for the driving, and the
`tobico-scripts` wrappers for the sandbox. They are what built Verkstead, stage
by stage, and they are what it replaces. Starting a piece of work stops meaning
*which of the three does this part* and starts meaning *open the workbench*.

**Where this stands.** The pipeline is proved by Verkstead's own test suite and
by nothing else yet: no repository has been taken through it end to end, and
the old three are still what executes this roadmap. The switch-over is written
down here ahead of being made, so that making it is a matter of following a
page rather than reconstructing a design — and the first real run belongs to
[stage 05](roadmaps/mvp/05-refinement.md).

The vocabulary in bold is the project's, defined once in
[CONTEXT.md](../CONTEXT.md).

## What it replaces

| Before | Now |
| --- | --- |
| `sandbox` / `work-sandbox` — bwrap around the whole of `~/src` | A **Sandbox** per **Conversation**: its **Worktree**, its Repo's git directory, its handoff directory, the **Agent Profile**'s claude pair, and nothing else of the machine |
| `agent`, `grilling`, `next-stage`, `next-tasks` — one wrapper per thing you might start | One **Conversation**, which runs through Draft → Grilling → Direction → Implementing → Wrapping → Done |
| `roadrunner` — a terminal per run, driving `.tasks/` and `docs/roadmaps/` | The orchestrator, driving the same two files off the Repo, with the run visible on a **Timeline** instead of scrolling past |
| roadrunner's interruptions | **Interruptions** and their **Remedies**, as Events you answer from the workbench or your phone |
| askance — one queue of Question Sets for the machine | **Question Sets** on the Timeline of the Conversation they were asked from |
| The skills installed under `~/.claude/skills` | **Skills** shipped inside the binary and mounted read-only over the sandbox's, so a session's behaviour is the product's |
| A gate at every commit | No commit gates. Review consolidates in the wrap-up, per pull request |

What stays: **askance is a separate, maintained product**, and the
`tobico-skills` skills stay installed for ordinary terminal work in
repositories Verkstead is not driving. What retires is roadrunner and the
wrappers that launched it — see [The old tools](#the-old-tools).

## Getting it running

Nothing has been released under this name yet, so the flake is the whole of the
install. On a NixOS host, import it and enable the service:

```nix
services.verkstead = {
  enable = true;
  watchedPaths = [ "/home/you/src" ];
  home = "/home/you";                 # for .gitconfig and .config/gh
  sandboxBinds = [ "verkstead=/var/cache/verkstead-cargo" ];
};
```

Three of those are worth understanding before the first Conversation, because
each is a boundary rather than a convenience:

- **`watchedPaths`** is what Verkstead may operate inside. There is no default
  and no scan; a **Repo** is registered only from within one, and a path that
  merely reads as inside one is refused.
- **`home`** is where `.gitconfig` and `.config/gh` are read from, and those
  two are mounted read-only into every Sandbox. Without them a session commits
  as nobody and has no GitHub login.
- **`sandboxBinds`** is the **Sandbox Configuration** — every entry is a hole
  in the boundary, which is why it is set here and not anywhere the workbench
  can reach. A bare path goes to every session; `name=path` goes only to
  sessions working in the Repo registered under that name.

The server binds loopback and speaks plain HTTP. Answering from a phone needs
HTTPS, which is `tailscale serve --bg 8422` in front of it — and push
notifications need that HTTPS to work at all.

Out of a checkout instead, which is the same server with `verkstead.db` in the
working directory, is [development.md](development.md#quickstart).

## A day's work

**Once per machine:** register the Repos you work in, and save at least one
**Agent Profile** — a claude home and config pair, and a default model. A
Conversation fixes a **Grilling Profile** and an **Implementation Profile**
before it starts; the same Profile may fill both, and separate ones are how the
two halves bill to separate accounts.

**Then, per piece of work:**

1. **New conversation**, against a Repo. Write the **Brief** — the markdown
   document the work starts from, and its first Event. The base commit defaults
   to the default branch's tip and is yours to override.
2. **Start grilling.** The branch and the **Worktree** are made here, and a
   grilling session opens in the Sandbox. What it wants to know arrives as
   Question Sets on the Timeline and, if you have subscribed, on your phone.
   Answer from wherever you are; the session waits.
3. **The Proposal.** The grilling ends by proposing a **Direction**, with a
   **Handoff** written for whoever builds the work. Picking the named Option
   accepts it. Every other way of answering — a different Option, your own
   words, or leaving it open — sends it back, and the session decides for
   itself whether to keep grilling or propose again.
4. **Choose the Direction:** inline, task list, or roadmap. One press, and the
   pipeline it names starts. A task list breaks the work into `.tasks/` first;
   a roadmap stages it into `docs/roadmaps/`; inline builds it in one go.
5. **It runs itself.** Each **Step** is one fresh session, ended when the file
   it turns on has gone from the Worktree *and* the commit removing it has
   landed *and* the session has gone quiet. Commits appear on the Timeline with
   their diffs. Where Verkstead cannot go on — a session that exited badly, or
   one that landed nothing — an **Interruption** opens, and the run holds until
   you pick **Retry** (with whatever you say passed into the fresh session),
   **take over manually**, or **abort**.
6. **The finish runs unattended.** The last Step pushes and opens a **draft
   pull request** per the target repo's `docs/agents/git-workflow.md`, and the
   Conversation moves to Wrapping. The PR is a pinned Event; its commits and
   comments are fetched through the host's `gh`.
7. **The wrap-up settles itself.** A fresh-context session reviews the PR and
   raises what it finds as a Question Set. Failing checks dispatch fix sessions
   — two failed attempts at the same check is where it stops and asks. New PR
   comments dispatch sessions that address them. The Conversation reaches
   **Done** when the checks are green, the review Set is answered, and nothing
   said on the PR is left unaddressed.
8. **Merging is yours.** Done means Verkstead has finished with the work, not
   that it is on `main`. Nothing in the pipeline merges anything.

**On a roadmap**, settling is also what starts the next **Stage**: a
Conversation of its own, on a branch stacked on the unmerged predecessor where
the repository's workflow records how, primed with the stage brief as its Brief
and Implementing from the first moment. A **Notice** on the Timeline says which
Stage started and where its branch went — or that the roadmap has no Stage left
to run. Nobody presses anything for either.

## What is different in practice

- **Questions belong to a Conversation.** There is no global queue to work
  through: a Set is on the Timeline of the work it came from, and it stays
  there, answered, afterwards. Nothing leaves a Timeline.
- **The checkout is not what gets worked in.** Every Conversation has its own
  Worktree under the State Directory, so two pieces of work in one Repo no
  longer take turns, and the checkout you have open in an editor is not what a
  session is editing.
- **A run that stops is a thing on a page**, not a terminal you have to find.
  The Interruption carries the evidence — which Step failed, how it ended, what
  git made of the Worktree, and the tail of what the session last said — read
  at the moment the run stopped and kept.
- **Review happens once, on the pull request.** This is what "no commit gates"
  buys: nothing pauses per commit, and everything you would have said there is
  said in the wrap-up instead.

## The old tools

`tobico-skills/roadrunner` and the `tobico-scripts` wrappers are left exactly
as they are: still on `PATH`, not deleted, and carrying no deprecation notice
in their own repositories. One person uses them and that person knows they are
retired, so a notice in a repository only they read would be ceremony rather
than warning.

Which also leaves them as the fallback while the switch-over is being made, and
that is the better reason not to touch them: they built this, up to and
including the stage that retires them, and a tool that still runs is worth more
than one removed the day its replacement first worked.
