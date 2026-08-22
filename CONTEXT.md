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
The one directory Verkstead keeps what it makes in — the Worktrees, the
installed Skills, the handoff directories, and whatever later stages need to put
somewhere. Beside the database by default,
because that is the directory a packaged unit is already given to write. Not a
Watched Path and not the same kind of thing: a Watched Path bounds what the
human may point Verkstead at, and this is Verkstead's own.
_Avoid_: data dir, work dir, scratch space, cache

**Sandbox**:
What a session runs inside: its Conversation's Worktree, the Repo's git
directory and the Conversation's handoff directory writable, the Agent
Profile's pair at `~/.claude` and `~/.claude.json`, the system and the Skills
read-only, and nothing else of the machine at all
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
One of the workflows Verkstead runs its sessions by — grilling, implementing,
breaking down and working a Step now, the rest as the stages that need them
arrive. Verkstead's
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
Event; nothing happens off it. It records the work rather than the watching:
looking at a session's Screen, and even a Hold, leaves no Event — the badge
says so while it matters, and the record keeps what was built.
_Avoid_: feed, log, history, activity stream

**Event**:
One entry in a Timeline — a Brief, agent output, a Question Set, a Handoff, a
commit, a task list, a stage list, a PR, an interruption, a Notice. Each shows a
summary in the
Timeline and its full self in the details pane. Task lists, stage lists and PRs
are **pinned**: a fixed set, with no manual pin or unpin.
_Avoid_: item, record, message, step

**Notice**:
The one kind of Event Verkstead writes on its own account: which Stage it
started and where that Stage's branch went, or that a roadmap has no Stage left
to run. No agent wrote it and nobody pressed anything for it, and there is
nothing to do about one — where an Interruption is a question left open, a
Notice is a decision already taken. It is what running unattended owes the
human: a decision made while nobody was watching is one they have to be able to
read afterwards.
_Avoid_: log line, info event, message, alert

**Transcript**:
The session's own record of its conversation — the agent's prose, its tool
use, its reasoning, and what was put to it — kept word for word as the agent's
backend wrote it, and rendered readable in the details pane. What summaries
and Interruption evidence draw from, falling back to the Capture when a
session left none.
_Avoid_: messages, chat log, session log (the backend's file, not Verkstead's
record), transcript-as-bytes (that is the Capture)

**Capture**:
The terminal bytes of a session, kept byte for byte, escapes and all — how it
looked rather than what it said. What quiet-detection listens to, what the
Screen replays, and the record of last resort for a session that left no
Transcript.
_Avoid_: transcript (that is the readable record), raw output, tape

**Screen**:
The live terminal view of a running session, held by Verkstead and shown in
the workbench — one screen however many devices watch it, sized by whoever
resized last. A session that has ended shows its Screen as it last stood,
read-only. Watching one commits the human to nothing.
_Avoid_: terminal, console, attach view, pane

**Hold**:
The human at a live session's keyboard. Begins with the first keystroke typed
into a Screen and ends only by being handed back — never by a timeout or a
dropped connection, because the machine resuming over a half-finished
intervention is worse than a stalled run. While it lasts Verkstead keeps
recording but ends nothing and advances nothing, and the Conversation carries
*blocked on you*; on hand-back the ordinary end-of-session rules judge
whatever the human left. Distinct from the *Take over manually* Remedy, which
is Verkstead stepping aside for good.
_Avoid_: takeover (the Remedy's word), pause, manual mode, lock

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

**Direction**:
How a Conversation's work gets built — **inline**, **task list** or **roadmap**
— chosen by the human in the workbench once the grilling has proposed wrapping
up. One of the three and never a mixture: the choice is which pipeline runs the
work, and a Conversation that had picked two would be two pieces of work. It is
also the state that choosing happens in, which is the same word on purpose:
Direction is where the Conversation is and the Direction is what comes out of
it.

Choosing is one press, and what follows from it is the pipeline it named
starting. The choice and the start stay separate things on the record — the
Direction is an Event and the work beginning is a move — but there is no second
button between them: the human has decided, and a Conversation sitting on a
settled Direction with nothing running would be waiting for nobody. All three
start as they are chosen — inline on a session that builds the work, a task list
on one that breaks it into `.tasks/` first, a roadmap on one that stages it into
`docs/roadmaps/` — and the Conversation is Implementing every way round, because
writing a plan for the work is the work when it is the work that was chosen.
_Avoid_: mode, strategy, plan, execution path

**Proposal**:
The grilling agent's closing move: a recommended Direction, the reasoning for it,
and the Option that means *go ahead*, carried as a block on one Question Set.
What can end a grilling rather than continue it — no button anywhere ends one.
Ordinary grilling Sets carry none.

Only picking the named Option accepts a Proposal and moves the Conversation to
Direction. **Every other way of answering sends it back**: another Option, an
answer in the human's own words, or the question left open. That is how the
human disagrees, and it is the whole way back — the session that proposed is
still holding the thread, and takes their Response to decide for itself whether
to keep grilling or propose again. A Proposal that was sent back is not in force
and is not what the chooser draws; the latest accepted one is.

The recommendation is marked in the chooser and never preselected: accepting the
Proposal settles that the work is understood, not which Direction it takes, and
the human may pick any of the three.
_Avoid_: wrap-up request, handover, recommendation (that is the part, not the
whole), final question

**Handoff**:
The document a grilling session writes before it proposes: everything it
settled with the human, written down for whoever builds the work. The other
half of the closing move, and the reason an inline implementation can be a
fresh session at all — the two run under different Profiles, and a session
cannot change the account it is running as, so what the grilling knows is
written down or it is gone.

Verkstead's document rather than the project's, so it is written outside the
Worktree — in a directory of the Conversation's own under the State Directory,
bound into every one of its Sandboxes. A handoff in the checkout would be a file
the next `git add -A` swept into the human's repository, and instructing an
agent not to commit something is worth less than the file not being there.

Taken onto the Timeline as an Event when the proposal is accepted, and taken
rather than copied: the Timeline holds the only one from then on. One per
grilling round — a proposal sent back leaves it where it is, to be rewritten
before the next one.
_Avoid_: handover document, context dump, summary, notes

**Step**:
One piece of unattended work a session is launched for and ended after: a task
of a Conversation's backlog, the breakdown that writes that backlog, the finish
that follows the last task, or an inline implementation, which is the whole of
the work in one Step. What is next is read from the Repo and nowhere else — the
lowest-numbered task file left in `.tasks/`, or `TODO.md` on its own — so the
Steps are the backlog's, and Verkstead keeps no list of its own to disagree with
it.

A Step is **done** when the file it turns on has gone from the Worktree *and* the
commit removing it has landed. A session reports through the repository, being an
ordinary interactive one, and a commit is the one report it cannot half make — a
file deleted but not committed is a session still mid-Step.

Its session is ended once the Step is done **and** the session has gone quiet for
a grace period, never on done alone: work does not always stop at the commit, and
output arriving puts the whole grace back on the clock. A session that keeps
talking is never ended. One Step per session and one session per Step — a fresh
context each time, which is what the backlog was broken into slices for.
_Avoid_: job, iteration, unit of work, stage (that is a roadmap's)

**Stage**:
One numbered entry of a roadmap, and a Conversation of its own: one branch, one
review unit, one pull request. Started by the Stage before it settling rather
than by anybody pressing anything — against the same Repo, under the same
Profiles, primed with the stage brief as its Brief, and Implementing from the
first moment, because the grilling that would have settled the work wrote the
brief. Its branch stacks on the unmerged predecessor where the target
repository records how, and comes off the default branch where it does not.

Done when its box in `ROADMAP.md` is ticked, which is the roadmap's own score
and is kept one Stage behind: the tick rides in the plan commit of the Stage
after it, so a Stage whose work has settled is still the box that says *in
progress* on this branch.
_Avoid_: phase, milestone, epic, step (that is a backlog's)

**Adopt**:
Take a roadmap the Repo already holds — written by the old tools, by hand, by
anything that was not this Verkstead — into the pipeline, by starting its next
Stage as a Conversation. The human's press stands in for the Stage before it
that would otherwise have started it, so there is no grilling and no Brief to
write: what they settle is the two Profiles and the base commit, and the stage
brief becomes the Brief. One Stage is the whole of what adopting starts, and all
it has to start — that Stage's own plan commit writes to the roadmap, so when it
settles the Stage after it begins the ordinary unattended way, and an adopted
roadmap is a staged one from there on. Never stacks: there is no predecessor
Conversation to stack on, and building on an unmerged branch is the human's
move, made by setting the base commit.
_Avoid_: import, attach, resume, take over, migrate

**Abandoned**:
What a roadmap in a registered Repo is when it has a Stage startable right now
and nothing driving it — the one state Adopt is offered for. Four things
together, read at the Repo's default branch tip: an unchecked box, a readable
brief for the lowest of them, no in-progress annotation naming a branch that
still exists, and the Stage's own slug branch not taken. A roadmap that is
finished, one already in flight and one whose next brief is missing are each not
abandoned and each draw nothing, because what the human can do something about
is the only thing worth saying. Read from the repositories every time it is
drawn and never stored, like the pinned stage lists — and with nothing to
dismiss one by, a roadmap's score being the repository's to keep: an unwanted
notice is silenced there, by ticking the box or annotating the stage.
_Avoid_: stale, orphaned, dormant, unmanaged, needs attention

**Interruption**:
Something Verkstead detected about an unattended run and cannot resolve itself: a
session that exited badly, or one that ended having landed nothing. It is an
Event on the Timeline like any other, and what makes it different is that it is
**open** — the run does not advance past one, and its Conversation carries
*blocked on you* until a Remedy is chosen. At most one is open per Conversation.

It carries the **evidence**, which is what makes the choice answerable without
opening a terminal: which Step failed, how it ended, what git made of the
Worktree, and the tail of what the session last said. All four are read at the
moment the run stopped and kept, because all four move on.
_Avoid_: error, failure, crash, incident, alert

**Remedy**:
One of the three things the human can do about an Interruption. **Retry** runs
the Step again in a fresh session, told whatever they wrote alongside — so "try
again but leave that one alone" reaches the agent that can act on it. **Take over
manually** stops Verkstead driving, so the human can take the Step on themselves.
**Abort** ends the run.

In every case the repository is left exactly as the session left it: no Remedy
reverts, resets or stashes anything, which is what makes taking over one at all.
Aborting from here therefore keeps the Worktree, unlike aborting a Conversation.
_Avoid_: action, resolution, fix, recovery option

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
it is showing. A query whose rendering holds reader state must therefore
reconcile its re-reads, or be `static` where its payload cannot change
(ADR-0005).
_Avoid_: tick, refresh signal, ping, change event

**Update Notice**:
The banner the viewer shows when the server has learned that a newer release
exists than the one it is running, linking to the update instructions. Informs
only — nothing is installed on the human's behalf.
_Avoid_: upgrade prompt, new-version alert
