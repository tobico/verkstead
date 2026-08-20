# Verkstead

A single-user management platform for agentic coding. A web GUI drives
everything; a background orchestrator runs sandboxed coding sessions, puts
Question Sets and commits to the human, and works through task lists and staged
roadmaps unattended. The human answers from any device on the tailnet.

Terms are grouped by what they belong to. The workbench vocabulary is
Verkstead's own; the question-set vocabulary is inherited from askance and
still holds word for word, because the asking half is unchanged.

## The workbench

**Watched Path**:
A directory Verkstead is permitted to operate inside, configured in the
environment at installation. A security boundary rather than a convenience: any
filesystem operation on a path outside every Watched Path is refused, and Repos
are registered only from within one.
_Avoid_: project root, workspace, scan path, allowed directory

**Repo**:
A git repository registered with Verkstead from inside a Watched Path.
Conversations attach to one. Its files stay the source of truth for task lists
(`.tasks/`) and roadmaps (`docs/roadmaps/`) — Verkstead parses and renders
them, and never owns them.
_Avoid_: project, codebase, checkout

**Conversation**:
The core entity: a Repo, a base commit, a Brief, one branch and one Worktree.
Everything done about one piece of work hangs off it. Runs through Draft →
Grilling → Direction → Implementing → Wrapping → Done, and can be aborted from
any of them or reopened once Done. *Blocked on you* is a badge on an active
state, never a state of its own — where **Aborted** is a state of its own, off
the ladder rather than on it: every other state is somewhere the work has got
to, and aborting is the work stopping wherever it was.
_Avoid_: task, session, job, thread, ticket

**Worktree**:
The checkout a Conversation's work is done in, made when grilling starts along
with the branch it holds, and removed when the Conversation is aborted — the
branch outlives it, because a branch is cheap and may hold work worth reading.
Named for the Repo and the branch, and it lives in the State Directory rather
than inside a Watched Path: Verkstead made it, so it goes among Verkstead's own
things.
_Avoid_: checkout, working copy, sandbox (that's what runs *in* it), clone

**State Directory**:
The one directory Verkstead keeps what it makes in — the Worktrees, and
whatever later stages need to put somewhere. Beside the database by default,
because that is the directory a packaged unit is already given to write. Not a
Watched Path and not the same kind of thing: a Watched Path bounds what the
human may point Verkstead at, and this is Verkstead's own.
_Avoid_: data dir, work dir, scratch space, cache

**Sandbox**:
What a session runs inside: its Conversation's Worktree and the Repo's git
directory writable, the Agent Profile's pair at `~/.claude` and
`~/.claude.json`, the system and the Skills read-only, and nothing else of the
machine at all
— not even the checkout the Worktree was made from. The filesystem is the
boundary and the network is not: inside, it is the host's own, whole and
unfiltered, because what stops a session doing harm is that there is nothing
within reach to harm.
_Avoid_: container, jail, isolation, environment

**Sandbox Configuration**:
The extra writable binds a Sandbox gets beyond that surface — a build cache, a
package registry's — as one global set every Sandbox gets plus a per-Repo set
composed over it. Configured where the Watched Paths are rather than anywhere
the workbench can reach: every one of them is a hole in the boundary, and
widening a boundary is the installer's to do.
_Avoid_: sandbox settings, mounts, extra paths

**Skill**:
One of the workflows Verkstead runs its sessions by — grilling now, the
implementation and wrap-up ones as the stages that need them arrive. Verkstead's
own: shipped inside the binary, installed under the State Directory at startup
and mounted read-only over `~/.claude/skills`, so a session's behaviour is the
product's rather than whatever the machine or the account happens to keep. A
session is put inside one by the prompt it is started on, which names the Skill
above the Brief.
_Avoid_: prompt, instructions, plugin, workflow file

**Brief**:
The editable markdown document a Conversation starts from, and its first
Timeline Event. Freezes when grilling starts; a reopened round adds a new Brief
rather than editing the frozen one.
_Avoid_: description, prompt, spec, issue body

**Timeline**:
A Conversation's ordered record of what has happened to it, and the middle pane
of the workbench. Everything Verkstead and its agents do lands here as an
Event; nothing happens off it.
_Avoid_: feed, log, history, activity stream

**Event**:
One entry in a Timeline — a Brief, agent output, a Question Set, a commit, a
task list, a stage list, a PR, an interruption. Each shows a summary in the
Timeline and its full self in the details pane. Task lists, stage lists and PRs
are **pinned**: a fixed set, with no manual pin or unpin.
_Avoid_: item, record, message, step

**Agent Profile**:
A named coding-agent account Verkstead can run a session under: a claude home
directory and config file pair, a default model, and an agent-type
discriminator so other backends can slot in later. The pair is bind-mounted at
`~/.claude` / `~/.claude.json` inside the sandbox, which is what keeps accounts
separate.
_Avoid_: account, identity, persona, agent config

**Grilling Profile** / **Implementation Profile**:
The two Agent Profiles a Conversation fixes before grilling starts — one for
the grilling session, one for the implementation work. They are roles a Profile
is used in, not kinds of Profile: the same Profile may fill both. Distinct
Profiles are why an inline implementation is a fresh session rather than the
grilling session carrying on.
_Avoid_: primary/secondary profile, planner/worker, grilling agent

**Blocking Ask** / **Deferred Ask**:
The two ways an agent puts a Question Set to the human. A **Blocking Ask** idles
the session until the Response arrives, as every ask does in askance. A
**Deferred Ask** does not idle it: the Set waits in the Timeline and its
Answers are folded into a later session's prompt. Work blocks only on Questions
whose Answers affect work about to be done.
_Avoid_: sync/async ask, hard/soft question, urgent question

## Question Sets

**Question Set**:
A batch of Questions submitted together by one agent, with a Preface and a
title. The unit that lands on a Conversation's Timeline, gets answered, and is
archived. Reached through the Conversation it was asked from and nowhere else:
a second way in would be a second thing to keep true.
_Avoid_: request, batch, ticket

**Preface**:
The markdown context that accompanies a Question Set, giving the human
everything needed to understand the Questions without seeing the agent's
session.
_Avoid_: description, context, body

**Postscript**:
Optional markdown the agent closes a Question Set with, rendered in the
section that closes the page, above the set-level comment box it shares that
section with — open-ended invitations only: suggested discussion topics, or
whatever else the human might take up in their comment. Never carries a
decision, however small — anything decidable, even a trivial yes/no, is a
Question. Named in the table of contents like the Preface, and headed for
the box alone on a Set that closed without one. Not a Question: a blank
comment beneath it means nothing to add, never Unanswered.
_Avoid_: epilogue, closing, anything-else question, trailing decision

**Question**:
A single labelled decision put to the human. Carries an agent-supplied opaque
label (e.g. `Q7`), markdown prose text, and optionally Options and
Sub-questions.
_Avoid_: item, prompt

**Sub-question**:
A leaf Question nested one level under a Question, labelled by letter
(e.g. `Q7a`). Sub-questions never have their own Sub-questions.
_Avoid_: child question, part

**Heading**:
A Question carrying Sub-questions and no Options of its own. Its text heads
them rather than asking anything: it is drawn without a field, and no Answer
comes back for it — a Response carrying one is refused. Read off the shape,
never declared.
_Avoid_: group, parent question, section

**Option**:
One discrete choice offered on a Question or Sub-question, numbered `.1`,
`.2`, … Its text is a label rather than prose, so markdown in it is inline
markup only. At most one Option per question is the Recommendation.
_Avoid_: choice, answer option

**Recommendation**:
The Option the agent marks as its preferred answer (the grammar's `★`).
_Avoid_: default, suggestion

**Answer Table**:
A question's Options declared with tabular data, one row per Option, drawn
by the viewer as a table whose rows are the selectable Options — the row is
picked in place of a list entry, and the Recommendation and the selection
are marked on the row.
_Avoid_: options table, comparison table (the Guide's generic layout
advice), grid

**Answer**:
The human's resolution of one Question or Sub-question: a selected Option
and/or free text. A Question left without an Answer at submission is
Unanswered.
_Avoid_: reply, response (that's the whole Set)

**Unanswered**:
The explicit state of a Question the human chose not to resolve when
submitting. The agent must treat it as still open — typically because the
set-level comment redirects the discussion.
_Avoid_: skipped, blank

**Response**:
The submitted collection of Answers (and Unanswered markers) for a Question
Set, plus an optional set-level comment. What the waiting agent receives.
May contain zero Answers.
_Avoid_: submission, result

**Diff**:
The uncommitted changes (including untracked files) of the asking repo,
captured by the CLI at send time and attached to every Question Set, so code
approval can happen in the web UI.
_Avoid_: patch, changeset

**Diagram**:
A mermaid fence in a Preface, a Question or a Postscript, rendered visually
in the viewer.
Degrades to its readable source text whenever it cannot render.
_Avoid_: chart, graph, figure

**Gutter**:
The reserved left-hand area every section of a Set page keeps when the
window is wide, one shared width across the page. Structural marks live in
it — Question labels, an Answer Table's radio-and-number column, the Diff's
line numbers — and content starts at its right edge, so all reading shares
one left axis. Invisible in itself: a section with nothing to hang there
reserves it empty. Sub-question labels stay inline with their text rather
than hanging in it.
_Avoid_: margin, sidebar, left rail

**Guide**:
The agent-facing usage instructions shipped inside the CLI and printed by
it, so an agent needs nothing beyond the binary to learn how to ask. Split
into a core that every ask needs and Topics fetched when their task arises.
_Avoid_: help, manual, docs

**Topic**:
A task-scoped section of the Guide (e.g. gates), split out so an agent pays
its reading cost only when the task is at hand — at which point it is
required reading, never optional.
_Avoid_: section, chapter

**Archive**:
What Question Sets become once their Response is delivered (or once an orphaned
Set is manually archived). Permanent decision history, and no longer a place of
its own to browse: a settled Set stays on the Timeline it was asked from,
saying what became of it. Nothing leaves a Timeline.
_Avoid_: history, log

**Liveness**:
Whether an agent is currently connected and waiting on a Question Set
("agent waiting" vs "agent disconnected"). Display state only — never causes
automatic withdrawal.
_Avoid_: connection status, presence

**Nudge**:
The contentless signal telling an open viewer page that the pending world
changed — a Set arrived, was answered, or was archived — so the page should
look again. It says nothing about what changed; the page refetches everything
it is showing.
_Avoid_: tick, refresh signal, ping, change event

**Update Notice**:
The banner the viewer shows when the server has learned that a newer release
exists than the one it is running, linking to the update instructions. Informs
only — nothing is installed on the human's behalf.
_Avoid_: upgrade prompt, new-version alert
