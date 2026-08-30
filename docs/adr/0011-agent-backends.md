# Agent backends beyond Claude Code

Amends [ADR-0001](0001-blocking-cli-for-agent-integration.md): the blocking
CLI is no longer every backend's channel.

Verkstead runs its sessions on one coding agent, and the `AgentType`
discriminator has sat in the Profile with one value in it since the store was
written — "so a second backend slots in beside `claude` rather than having to
be migrated in underneath it". This decision spends that slot three times:
**Codex** (OpenAI's `codex`), **Grok Build** (xAI's `grok`) and **OpenCode**
(`opencode`) become backends a Profile can name, each at full parity — not
merely launchable with the Capture as the record, but driven through every
state with a Transcript reader and a usage-limit phrase of their own.

The launch line turned out to be the shallow coupling. What actually bound
Verkstead to Claude Code was everything around it: the skills mounted over
`~/.claude/skills`, the blocking ask that assumes an agent can hold a shell
command for hours, quiet detection calibrated to an inline terminal drawing,
the Transcript found by the session id Verkstead names at spawn, and the one
usage-limit sentence. Each of those gets a rule here rather than a special
case per backend.

## The ask channel is a property of the backend

The Blocking Ask assumes holding a shell command open for hours costs
nothing. That is true of Claude Code (a background command the harness wakes)
and of OpenCode (a synchronous shell tool that accepts any timeout the model
passes, holding no model turn open) — and false of Codex and Grok Build,
whose shell tools yield after seconds and have the model *poll* the running
command, each poll a full paid model turn. An ask held that way burns tokens
for hours and has documented wedge bugs.

So each backend asks on the channel it can afford:

- **Blocking** — Claude Code and OpenCode. `verkstead ask` as today.
- **Store-and-nudge** — Codex and Grok Build. `verkstead ask` stores the Set
  and returns at once, as `--deferred` does, and the agent ends its turn. When
  the Response lands and the asking session still runs, Verkstead types one
  line into its terminal — the channel Rescue already uses — telling it to
  fetch the answers with **`verkstead answers`**, a new command that prints a
  stored Set's Response. A session that has gone by then is not a lost
  Response: the answers fold into the next session's prompt the way answered
  Deferred Asks already do.

Store-and-nudge is the Deferred Ask machinery — the same stored Set, the same
folding rule — with one thing added underneath it: **a store-and-nudge Set is
a state of its own**, and not a Deferred Ask wearing a nudge.

It has to be, because the two mean opposite things to everything that reads
them. A Deferred Ask is excluded from `store::unanswered_set_since`
(`d.set_id IS NULL`) precisely because it is a Set nobody is idling on, and
both halves of the run read that exclusion: the enders through
`runner::asking`, and Rescue through `runner::open`. A store-and-nudge Set
stored as a deferred one would therefore look like a session that went quiet
having asked nothing — ended by the quiet grace, and prodded twice and then
stopped by Rescue, before the human had answered and so before there was any
session left to nudge. The new state is **counted as open** by both, and the
session waits on its answers exactly as a blocking one does.

Which is also what keeps `--deferred` meaning what it means on these
backends: a Set nothing is idling on, and one the nudge leaves alone.

A session learns which channel is its own from the Guide: Verkstead sets the
agent type in the sandbox environment, and `verkstead guide` prints the
asking instructions for that backend. One Guide, tailored at print time —
not a fork of the skills per backend, and not a longer prompt.

## Quiet is read off the screen for a full-screen TUI

All three new backends draw full-screen TUIs, and none is confirmed
byte-silent when idle — spinners and frame-rate-matched renderers keep
writing to the terminal. The three-second byte-quiet rule that idling, Rescue
and session-ending stand on was calibrated to Claude Code's inline drawing
and does not carry.

For these backends, idle is judged by **parsing the drawn screen** — the
Screen Verkstead already holds for the live view — for that backend's
at-the-prompt state. The mark is one signature constant per backend, the same
bargain the usage-limit phrase already makes: the wording is the backend's
and will move, so it is kept in one place and costs one edit when it does.

The three-second byte-quiet mark does **not** count as idle on a TUI backend:
a TUI that falls silent for a moment mid-turn would read as idle and be
rescued out from under its own work. A **long** byte-quiet does, and has to.
A signature that has drifted reads as a session that never goes idle, and
nothing in the run catches that on its own: Rescue's precondition is quiet, so
`until_it_will_not_ask` never reaches the rescue below it; every ender —
`quiet_and_nothing_asked`, `committed_and_quiet`, `nothing_else_and_quiet` —
gates on the same clock; and no session carries a cap on its life. A drifted
signature would otherwise be a session running for ever, holding its Worktree,
with the backlog stopped and nothing in front of the human to say so.

So a TUI backend keeps byte-quiet as a **long-stop**, measured in minutes
rather than seconds — well past any gap a redrawing TUI leaves — and a session
that crosses it is idle whatever its screen says. A drifted signature then
lands in front of the human as the ordinary would-not-ask stop: one slow round
rather than never.

Claude Code stays on the three-second byte-quiet, which works and stays
measured on what it was calibrated for.

## The skills move to a neutral path

The bundled skills mount at **`/verkstead/skills`** for every backend, Claude
Code's sessions included, and the prompts and the skills' cross-references
name that path. `~/.claude/skills` was Claude's own place; a neutral one is
nobody's, and it sits beside `/verkstead/bin`, which the sandbox already
makes. (Grok Build and OpenCode discover Claude-style skill directories
natively, and Codex reads whatever path the prompt names — the mount point
never needed to be Claude's, only *a* path the prompt could say.) The skills'
own Claude-specific wording — background-shell advice and the like — is
generalised per backend as part of the move.

**The hiding the old mount did is kept, deliberately.** Landing at
`~/.claude/skills` covered whatever the account had there, and that was the
point of the path as much as the mounting was: an account's own skills are
hidden rather than merged with, because a Profile is an account to run as
rather than a second opinion about how to work. A mount that has moved away
covers nothing, and a Claude session would find the human's own skills again
— including an older fork of the ones Verkstead ships, which is the case the
hiding was for. So an **empty directory is bound read-only over
`~/.claude/skills`** beside the new mount, and each new backend's own
discovery path is covered the same way as its stage lands — except where that
path is inside the account home itself, as Codex's is. Covering `~/.codex/skills`
would hide the skills codex ships as well as the ones the account added, and the
home is the whole of what a Codex Profile names, so a Codex home is left as the
account keeps it.

## A Profile is one home directory, except Claude's pair

Each new backend keeps its whole account under one relocatable home — Codex
under `~/.codex`, Grok Build under `~/.grok`, OpenCode under its XDG config
and data directories. A new-type Profile therefore stores **one home
directory**, bound where that backend expects it (OpenCode's XDG paths
pointed into it); Claude keeps its existing directory-plus-config-file pair,
already stored and already working. The Profile form takes a per-type shape,
and offers a backend only once its stage has landed — a type that cannot
launch would be a lie in a picker.

## Unattended is the product's promise

Verkstead passes each backend's approval-bypass flags itself:
`--dangerously-bypass-approvals-and-sandbox` for Codex (its own sandbox
refuses to start inside bwrap, and bwrap is already the boundary — the flag
was `--yolo` when this was written and codex has since dropped that spelling),
`--always-approve --sandbox off` for Grok Build for the same reason, the
permission configuration for OpenCode — and Claude Code moves to the same
rule, carrying `--dangerously-skip-permissions`, rather than the Profile's
own settings being what keeps a run unattended. What stops a session doing
harm is the Sandbox, which is unchanged; a backend stopping to ask approval
mid-run stalls a run nobody is watching.

The host provides the binaries, as it provides `claude` today: the installer
puts `codex`, `grok` and `opencode` on the system profile the sandbox
already reads, and a Profile whose binary is missing fails at session start,
named in the Capture.

## Parity where the backend gives less

- **Transcript discovery.** Grok Build takes a session id at launch as
  Claude does, so its log is named. Codex and OpenCode take none, so theirs
  is *found*: the log that appears in the account's session store for the
  session's worktree after launch, matched on working directory and start
  time. ADR-0006's rules are unchanged — lines stored verbatim, parsed at
  render time, the Capture the complete record wherever no log is found.
- **Usage limits.** The phrases ship as known — Codex's is confirmed — and
  the rest are filled in when first observed; until then such a stop lands
  as an ordinary stall, caught by the ordinary rules. OpenCode retries
  provider limits internally before surfacing anything, so its phrase may
  stay empty for a long time, and that is fine.
- **No account switching**, unchanged: an exhausted Profile is a wait on any
  backend, never a reason to spend a different one.
