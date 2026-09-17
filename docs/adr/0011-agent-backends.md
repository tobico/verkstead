# Agent backends beyond Claude Code

Amends [ADR-0001](0001-blocking-cli-for-agent-integration.md): the blocking
CLI is no longer every backend's channel.

Amended (2026-09-17): **the account is built rather than joined.** A session
is given a root of Verkstead's own seeded from the account by an allowlist, not
the account's directory whole; the cover over `~/.claude/skills` goes with it,
and the bypass that keeps a Claude run unattended is written into the root as
well as passed on the line. See the three *Amended* paragraphs below, and the
[built-roots roadmap](../roadmaps/built-roots/ROADMAP.md).

Amended (2026-09-18): **Codex, Grok Build and OpenCode are built by the same
rule, and every Profile has a memory switch.** Transcript discovery reads the
store where it really is on the host: the account's with memory on, the root's
with it off. See *As built for Codex, Grok Build and OpenCode*.

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

Amended: **the two blocking backends share a channel and not a mechanism, so
the Guide's *Running the ask* is per backend rather than per channel.** Claude
Code makes the call a background one and is woken when it returns; opencode
runs it synchronously inside the model's turn. Which kinds of ask a backend
has stays the channel's, because that is the whole of what a channel decides.

And what an OpenCode session has to be told is **pass a large timeout**,
measured against opencode 1.18.25: the shell tool takes any positive timeout
in milliseconds and holds the command for it, but a call that passes none is
killed at its default — two minutes, which is what its own description tells
the model — and `verkstead ask` waits on a human with a phone. So Verkstead
does both: the Guide says to pass one, and the sandbox raises the default
underneath it with `OPENCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS`, so a
session that ignored its Guide still holds. Measured on that release against a
stand-in provider: a `verkstead ask` held 170 s and came back with the Response
as its tool output seconds after the human answered; the same call with no
timeout and no variable was killed at exactly 120 000 ms; and with the variable
set and no timeout passed it held past the default and came back.

**A held ask is a session at work, and not a quiet one.** opencode animates the
dial beside its `esc interrupt` label for as long as the shell tool holds the
command — writing every 20–40 ms and leaving the label standing — so a session
waiting on the human reads *at work* by this backend's own signature, and
neither the enders nor Rescue nor the byte-quiet long-stop is anywhere near it.
The unanswered Set that holds such a session open is therefore the second line
of defence here rather than the only one, which is the right way round: it is
what stands the day the label moves or the renderer settles, and it is what
Verkstead's own suite proves against a session that is quiet and shows no label
at all.

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

Amended: **a signature reads one of two ways, and which way is a fact about
the backend rather than a choice.** Both assumptions above were checked
against the real codex when its stage was built, and both came out the other
way round for it:

- **Its waiting frame is its working frame.** The screen codex leaves when its
  turn is over and the screen it draws mid-turn differ by one line — the
  `Working (12s • esc to interrupt)` status line, which is there while it
  works and gone when it is waiting. The composer, its `Ask Codex to do
  anything` placeholder and the bar under it stand in both, so there is no
  at-the-prompt state to parse. What the constant can be is the *at-work* line,
  and standing then says the opposite of what it says above.
- **It is byte-silent at its prompt.** Not one byte once the frame settles,
  where mid-turn it repaints every 33 ms without a gap.

So the constant per backend stands, and what it says about the session is the
backend's too: a prompt line *standing* says stopped, an at-work line *going*
says stopped. The second is half an answer rather than a whole one — the line
is equally missing from the frame of a session that has drawn nothing yet, and
from every frame of a session drawing a wording this build has never seen — so
the ordinary three-second quiet is asked for beside it. That is what keeps a
drifted at-work phrase from reaping a session mid-work: a TUI at work
repaints, and one that is repainting is never quiet. Where the phrase drifts
the other way and never goes, the long-stop below is still what catches it,
unchanged.

The three-second mark counting for one reading and not the other is not a
softening of the rule above. It is that rule's own reasoning applied to a
different screen: a moment's silence mid-turn says nothing about a session
whose at-work line is standing, and it is only ever consulted once that line
has gone.

Amended again: **Grok Build reads the same way round as codex**, measured
against grok 1.0.13 on a hundred-column terminal with a stand-in API behind it.

- **Its waiting frame is its working frame.** The composer, its `❯`, the
  `grok-4.6 · always-approve` label on its border and the `Shift+Tab:mode` and
  `Ctrl+x:shortcuts` hints beside it stand in both. What is there only while a
  turn runs is the live status line — `⠧ Responding… 5.7s … [stop]` — and the
  `Esc:cancel` hint on the row under the composer, which go and come together:
  across a turn sampled once a second, both were in every working frame and in
  none of the resting ones. The constant is the hint, the hints being the row
  grok draws at the foot of every frame where the status line is drawn only
  mid-turn.
- **It is byte-silent at its prompt**, and emphatically: not one byte in ninety
  seconds of sitting there. Mid-turn the widest gap between reads was 208 ms
  once it had drawn its first frame, so the three-second quiet asked for beside
  the hint is never met while it works.

So both backends that ship a signature so far read by an at-work line, and the
at-the-prompt reading stands unused for the backends that will draw a prompt of
their own.

Amended a third time: **OpenCode reads that way round as well**, measured
against opencode 1.18.25 on a hundred-column terminal with a stand-in provider
behind it.

- **Its waiting frame is its working frame.** The composer, the `Build auto ·
  <model>` label on its border and the `tab agents` and `ctrl+p commands` hints
  stand in both. What differs is the status bar at the foot of the frame: while
  a turn runs it is a progress dial and an `esc interrupt` label, and at rest it
  is the project's path instead. Across two turns of one session sampled once a
  second — a tool call and then a streamed reply, twice — the label was in every
  working frame and in none of the resting ones. The constant is the label
  rather than the dial in front of it, the dial's cells filling and emptying
  every frame where the label does not move.
- **It is byte-silent at its prompt**: not one byte in the 106 seconds it was
  left sitting there. Mid-turn the widest gap between reads was 86 ms once it
  had drawn its first frame, so the three-second quiet asked for beside the
  label is never met while it works. Before that first frame the widest gap was
  1.4 s, which is the startup the quiet is there to cover.

So every backend that draws a screen has now been measured and every one of them
reads by an at-work line. The at-the-prompt reading stands unused: what it is
there for is the backend that turns out to differ, and none has.

**And opencode takes the alternate screen, with no flag to keep it inline.**
codex and grok each take `--no-alt-screen` because the Capture is the record of
what a session did and an alternate screen is a record thrown away as the
program leaves it. opencode's help offers no such flag, and `\e[?1049h` is
among the first bytes it writes. Two consequences, both measured rather than
reasoned about:

- **The Screen still reads.** The screen model already tracks which buffer is in
  front, so the idle judgement above and a human watching a live session both
  see the frames opencode is drawing.
- **The Capture replayed holds none of the session.** A clean exit writes
  `\e[?1049l`, and the grid a replay of the whole Capture then leaves is what
  was on the ordinary buffer: opencode's farewell banner, naming the session's
  id and the command to resume it, and nothing of the conversation. Every byte
  is still in the Capture — what is gone is the grid they drew. So for this
  backend the record a human reads back is the session store rather than the
  Capture, which is what the Timeline draws from anyway.

**The inline alternative is `--mini`**, the minimal interface opencode offers:
it writes no `\e[?1049h` at all, and it carries the same `esc interrupt` label
in a status bar of its own, so the signature reads under either. It is not
reached for here, because what it costs is the interface a human attaching to
the Screen gets and what it buys is a record this backend does not depend on.
It is what to reach for the day the Capture has to be that record.

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
path is inside the account home itself, as Codex's and Grok Build's both are.
Covering `~/.codex/skills` or `~/.grok/skills` would hide the skills those
programs ship as well as the ones the account added, and the home is the whole
of what a Profile of either type names, so such a home is left as the account
keeps it.

Amended (2026-09-17): **the cover goes, because there is nothing under it.** A
session's `~/.claude` is now built rather than joined — see the next section's
amendment — and a built root holds no skills of the account's to hide. The
hiding this paragraph argued for is kept by omission instead: what is not in
the allowlist is not in the root.

## A Profile is one home directory, except Claude's pair

Each new backend keeps its whole account under one relocatable home — Codex
under `~/.codex`, Grok Build under `~/.grok`, OpenCode under its XDG config
and data directories. A new-type Profile therefore stores **one home
directory**, bound where that backend expects it (OpenCode's XDG paths
pointed into it) — the amendment below gives a session a root built out of it
instead; Claude keeps its existing directory-plus-config-file pair,
already stored and already working. The Profile form takes a per-type shape,
and offers a backend only once its stage has landed — a type that cannot
launch would be a lie in a picker.

Amended (2026-09-17): **what a Profile stores is unchanged; what a session is
given of it is built.** The first Windows install to run a real session showed
what joining the account whole costs: the human's plugins and hooks ran in
every session and failed on every tool call, a session wrote a path inside the
sandbox's ephemeral storage into the human's plugin registry, an in-session
login and an in-session prompt rewrote the human's real files, and every
repository's transcripts were readable — and this repository's own Linux
sessions were doing the same, unnoticed, through the read-write bind. So a
session runs in a **built root** under the Conversation's profile, seeded by an
allowlist that is the same rule for every harness: the **credentials**, linked
so a login from inside lands in the account; the **memory store** — Claude's
entries under `projects/` for the Repo and its Worktree and no other
repository's, Codex's and Grok's `sessions/`, OpenCode's data directory — shared
when the Profile's memory switch says so, on by default; a **configuration
file Verkstead writes**, carrying only what names a model provider; and an
**instructions file** from the settings page, in place of the human's own
global one. Nothing else of the account is there. Claude's `.claude.json` is
copied rather than linked, pre-seeded with the Repo's trust, and written back
at session end as ADR-0014 decided, because a re-login from inside a run has
to reach the account. ADR-0014 built that write-back for Windows's hard links
alone; a copy follows nothing on any platform, so it now runs on Linux and a
Mac too.

**As built for Claude**, the allowlist and the write-back came out narrower than
the paragraph above in three places:

- **The written `settings.json` carries two keys of the account's:
  `apiKeyHelper` and `env`**, where the account's own file has them, beside
  `skipDangerousModePermissionPrompt`. Those two are how an API-key login
  reaches the model. Nothing else is carried — no `hooks`, `enabledPlugins`,
  `permissions` or `statusLine` — because an allowlist stays closed when Claude
  adds a key, and a denylist would not. An account with no `settings.json`, or
  one that does not parse, still gets the bypass key. The file is not written
  back.
- **The `.claude.json` copy has `mcpServers` taken out**, at the top level and
  under each `projects` entry. Those are the human's own MCP servers, the same
  leak as plugins.
- **The write-back of `.claude.json` is a merge by key, not a copy of the
  whole file.** A copy never shares an identity with the account's file, so
  the identity check alone would write it back after every session. Sessions
  run in parallel, so the last to end would undo what another session, or the
  human's own `claude`, wrote in the meantime; and a whole-file write-back of a
  copy without `mcpServers` would delete the human's servers. So the copy as
  given is kept as a baseline. At session end, only the top-level keys and the
  single `projects` entries whose value changed from that baseline are written
  into the account's file as it is by then, and a key the session removed is
  removed. `mcpServers` is never written or removed, at either level. The trust
  seeded into the copy reaches the account only in an entry the session itself
  changed. A session that changed nothing leaves the account's file byte for
  byte as it was. This narrows ADR-0014's whole-file write-back, and the human
  approved it.

Rejected: launch flags — `--setting-sources`, `--settings` — which close the
inheritance and leave the write-through open; `--safe-mode`, which takes the
Repo's own `CLAUDE.md`, the skills and MCP with it; and building Claude's root
alone, which the human turned down for consistency.

**As built for Codex, Grok Build and OpenCode** (2026-09-18), each allowlist
was read off a real install. Five things are recorded beside the paragraph
above:

- **The memory switch is on every Profile, Claude's included**: a `memory`
  column beside the account, added in place by a migration, on by default and
  on for every Profile that already existed. Off, nothing of the account's
  store is made or joined, and each of its directories is the root's own and
  empty — for Claude, an empty `projects/`. The login and the written
  configuration are the same either way.
- **Codex's store is `sessions/` and `memories/`, and none of its databases.**
  The human chose `memories/`, which holds `MEMORY.md` and
  `memory_summary.md`, beside the rollouts. `archived_sessions/` is not in it,
  and nor is any SQLite database at the top of `~/.codex` — `state_5.sqlite`,
  `memories_1.sqlite`, `thread_history_1.sqlite`, `logs_2.sqlite`,
  `goals_1.sqlite`, `queue_1.sqlite` in codex 0.154. A database linked a file at a
  time loses its write-ahead-log siblings and will not open, so each is the
  session's own and starts empty. A fresh state database beside a shared `sessions/` does no
  paid work: codex fills it from the rollouts as it starts, and the `memories`
  feature, which would summarise them with a model, is off unless `[features]`
  turns it on, which the written file never carries. `codex login` writes
  `auth.json` in place, so a link stays the account's file. The written
  `config.toml` carries `model_provider` and `model_providers`.
- **Grok Build's store is two directories, not the files the roadmap named.**
  Read off grok 1.0.13: `memory/` holds a global `MEMORY.md` and a directory per
  repository, each with a `MEMORY.md` and an `index.sqlite`; `sessions/` holds
  the session logs and a `session_search.sqlite`. Both databases run in
  write-ahead-log mode, so `index.sqlite` is never linked on its own: `sessions/`
  and `memory/` are joined whole, each database with its siblings. The human
  agreed. Grok has no one key that names a provider, so its written
  `config.toml` carries what its configuration reference gives to reaching a
  model and signing in to one: `model`, `model_providers`, `auth_provider`,
  `auth`, `grok_com_config`, and `endpoints.models_base_url` and
  `endpoints.models_list_url`. **And its login is a copy on Linux.** grok saves
  `auth.json` only by renaming a file over it, and a bind refuses the rename
  (`grok logout` inside fails with *Resource busy*), so on Linux the login is
  copied in, owner-only, and merged back by login scope at session end with the
  merge `.claude.json` uses. The human chose this. On a Mac and on Windows the
  rename replaces the link, and the ordinary write-back hands it back. Because a
  Linux session's HOME is the server's own, a root bound at `~/.grok` would hide
  `~/.grok/bin`, where xAI's installer puts `grok`, so read-only `PATH`
  directories under a root's name are bound again after it.
- **OpenCode's data directory is joined whole with memory on**, as decided: it
  holds `opencode.db` with its `-wal` and `-shm` siblings, checked on opencode
  1.18.30, and the login, `mcp-auth.json` and logs travel with it. With memory
  off it is the root's own, `auth.json` alone is linked in, and the database
  starts empty; `opencode auth logout` rewrites `auth.json` in place, so the
  link stays the account's file. The config directory is built either way and
  holds only an `opencode.json` carrying `provider`, read from `opencode.json`
  and then `opencode.jsonc`, comments allowed, the later file's keys winning.
- **Discovery reads where the store really is on the host, not the built root.**
  The roadmap said every transcript reader would look in the session's root.
  That holds only with memory off, where the root under `homes/<id>` is the one
  place the log is. With memory on, a Linux join is a bind inside the session's
  namespace, so the root on the host holds an empty directory and the log is in
  the account. So Claude's glob of `projects/`, Codex's rollout match, Grok's
  named session and OpenCode's database each look in the account's store when
  memory is on, and in the root on the host when it is off.

A login made inside where the account had none is handed to the account at
session end on all three platforms, for every harness. And the Windows grant
follows each allowlist rather than Claude's: an entry on each built directory,
one on each directory joined into it, and one on a linked login, and none on an
account directory as a whole — an OpenCode data directory joined whole being a
joined directory, granted as one. The check for files on another volume names
the login, which is the only thing a root joins by hard link.

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

Amended (2026-09-17): **the flag alone is not enough on a fresh account.**
Claude Code asks once, interactively, before it honours
`--dangerously-skip-permissions`, and records the answer in the account's
`settings.json`; a fresh account has never answered, and a session on a
pseudoconsole with nobody at it parked there for good. Every Linux session
here had an account that answered once by hand, which is why it was never
seen. So the answer is written into the built root's `settings.json` by
Verkstead — `skipDangerousModePermissionPrompt` — beside the flag on the line,
the way Codex's Worktree trust is already said on its line. `--print` was
suggested and is not the fix: the Screen, the Hold, quiet detection and the
Rescue all stand on the terminal. **`bypassPermissionsModeAccepted` is not
seeded** into the `.claude.json` copy, though the roadmap first named it:
Claude Code 2.1.268 moves that key out of `.claude.json` and into
`settings.json` as it starts, and the settings key already answers the same
consent.

Amended: **OpenCode's bypass is `--auto` on the launch line**, rather than the
permission configuration this paragraph first named. So every one of the four
says its approvals in the same place — one flag apiece on the line Verkstead
builds — and nothing about approvals lives in the sandbox's environment, where
the two things this backend *is* told there are both about something else: the
name of its store and how long its shell tool holds a command. opencode has no
sandbox of its own to switch off, so the flag is the whole of it.

The host provides the binaries, as it provides `claude` today: the installer
puts `codex`, `grok` and `opencode` on the system profile the sandbox
already reads, and a Profile whose binary is missing fails at session start,
named in the Capture.

## Parity where the backend gives less

- **Transcript discovery.** Grok Build takes a session id at launch as
  Claude does, so its log is named. Codex and OpenCode take none, so theirs
  is *found*: the log that appears in the account's session store for the
  session's worktree after launch, matched on working directory and start
  time. (Amended 2026-09-18: in the account's store where the Profile shares
  its memory, and in the built root on the host where it does not — see *As
  built for Codex, Grok Build and OpenCode*.) ADR-0006's rules are unchanged — lines stored verbatim, parsed at
  render time, the Capture the complete record wherever no log is found.

  **Amended: OpenCode's store is a database, not a file of lines**, read off
  opencode 1.18.25. The account keeps one SQLite database under its data
  directory — the file whose name Verkstead pins, so a beta install's is not
  a second guess — in write-ahead-log mode, with a row per session and a row
  per record within it, and every record's payload JSON text. The plan above
  expected JSON files appended to; they are gone.

  What survives is every rule and none of the mechanism. *Which* session is
  Codex's question asked in SQL rather than off a first line: the row whose
  recorded directory is this Conversation's Worktree and which was created at
  or after launch, newest first, and the session opencode started for itself
  rather than one it started under that. *Following* is the same bargain in
  the store's own terms — the cursor is the highest record sequence already
  taken, and each poll takes what has arrived past it. And *storing* is
  ADR-0006 unchanged: the payload reaches the Transcript byte for byte, with
  opencode's own kind and the sequence around it, parsed at render time.

  The database is opened read-only while opencode writes it, and a poll that
  cannot read is a poll that looks again — the store is not there for a
  session's first seconds, and the writer holds the lock from time to time
  after that. **A store this build cannot read leaves the session
  Capture-only**, said in the log rather than silently: the layout is
  opencode's own and moves between releases, and none of that may fail a
  session when the Capture is a complete record on its own.
- **Usage limits.** The phrases ship as known — Codex's is confirmed, and so
  is the wording Grok Build gives a free account — and the rest are filled in
  when first observed; until then such a stop lands as an ordinary stall,
  caught by the ordinary rules. OpenCode retries provider limits internally
  before surfacing anything, so its phrase may stay empty for a long time, and
  that is fine. **A backend with no phrase is skipped rather than matched
  against nothing**: the matcher compares the opening of a line against the
  phrase, so an empty one matches every line there is.
- **No account switching**, unchanged: an exhausted Profile is a wait on any
  backend, never a reason to spend a different one.
