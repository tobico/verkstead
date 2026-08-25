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
Grilling → Implementing → Wrapping → Done — a roadmap Conversation passes
straight from Grilling to Wrapping, its building belonging to its Stages — and
can be aborted from any of them or reopened once Done. There is one move back
down the ladder: a wrap-up whose review split its findings out into a backlog
returns to Implementing to build it, and its finish step wraps up again on the
pull request it already had, reviewed afresh. *Blocked on you* is a badge on an active
state, never a state of its own — where **Aborted** is a state of its own, off
the ladder rather than on it: every other state is somewhere the work has got
to, and aborting is the work stopping wherever it was.
_Avoid_: task, session, job, thread, ticket

**Worktree**:
The checkout a Conversation's work is done in, made when grilling starts along
with the branch it holds, and removed when the Conversation is aborted — the
branch outlives it, because a branch is cheap and may hold work worth reading.
A reopened Conversation keeps the one it has; where the directory has gone, one
is checked out again on the branch that was worked, which is the only time a
Worktree is made without a branch being made with it.
Named for the Repo and the branch, and it lives in the Data Directory rather
than inside a Watched Path: Verkstead made it, so it goes among Verkstead's own
things.
_Avoid_: checkout, working copy, sandbox (that's what runs *in* it), clone

**Data Directory**:
The one directory Verkstead keeps what it makes in — the database, at
`verkstead.db` inside it, the Worktrees, the installed Skills, the handoff
directories, the settings files it is told the human's credentials and identity
in, and whatever later stages need to put somewhere. Said once, as
`--data-dir`, and the working directory when nothing says otherwise; everything
in it is named by Verkstead rather than by whoever started it. Not a Watched
Path and not the same kind of thing: a Watched Path bounds what the human may
point Verkstead at, and this is Verkstead's own.
_Avoid_: state directory, work dir, scratch space, cache

**Sandbox**:
What a session runs inside: its Conversation's Worktree, the Repo's git
directory and the Conversation's handoff directory writable, the Agent
Profile's pair at `~/.claude` and `~/.claude.json`, the system, the Skills and
the Verkstead executable read-only, and nothing else of the machine at all
— not even the checkout the Worktree was made from. The filesystem is the
boundary and the network is not: inside, it is the host's own, whole and
unfiltered, because what stops a session doing harm is that there is nothing
within reach to harm. The `verkstead` a session asks with is the running
server's own image, first on the `PATH` inside, so the CLI a session asks with
and the server it asks are one build and cannot disagree about a schema.
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
own: shipped inside the binary, installed under the Data Directory at startup
and mounted read-only over `~/.claude/skills`, so a session's behaviour is the
product's rather than whatever the machine or the account happens to keep. A
session is put inside one by the prompt it is started on, which names the Skill
above the Brief.
_Avoid_: prompt, instructions, plugin, workflow file

**Brief**:
The editable markdown document a round of a Conversation starts from, and the
first Timeline Event. Freezes when its round's grilling starts; a reopened round
adds a new Brief rather than editing the frozen one, so a Conversation has one
Brief per round and the newest is the one being written. What is *not* the
human's again on a reopened round is the branch and the base commit: the branch
has been worked, and the second round carries on from what is on it.

Written where it is read: while its round is drafting, the Brief on its card
*is* the field — raw markdown, always open, keeping itself whenever the typing
stops for a moment and whenever the field is left, and saying nothing about it
either way.
There is no Edit, no Save and no word about saving, because there is no other
thing the Brief could be doing while it is a draft. Once it freezes it is the
server's rendering of it and nothing else.

While it is still a draft its card carries the whole of the Conversation's
setup under it — the branch, the base commit, both Pairings and the readiness
verdict — because setting the work up and kicking it off are one act, and both
belong where the work is read. Every one of those freezes at the same moment
the Brief does, so once grilling starts the card is the Brief alone; on a
reopened round the branch and the base commit are frozen already, and what the
card carries under the new Brief is the Pairings.
_Avoid_: description, prompt, spec, issue body

**Timeline**:
A Conversation's ordered record of what has happened to it, and the middle pane
of the workbench. Everything Verkstead and its agents do lands here as an
Event; nothing happens off it. It records the work rather than the watching:
looking at a session's Screen leaves no Event, and neither does typing into one
— the record keeps what was built rather than who was there.
_Avoid_: feed, log, history, activity stream

**Event**:
One entry in a Timeline — a Brief, agent output, a Question Set, a Handoff, a
commit, a task list, a stage list, a PR, a Notice. Each shows a summary in the
Timeline and its full self in the details pane. Task lists, stage lists and PRs
are **pinned**: a fixed set, with no manual pin or unpin.
_Avoid_: item, record, message, step

**Commit Summary**:
The agent-written account a code commit carries as its message body — a delta
Diagram first, prose after — kept by the sweep with its trailers stripped,
rendered above the diff in the commit's details pane, and clamped to a prose
snippet on its Timeline card. Written for commits that deliver work; pure
bookkeeping commits carry none, and a commit without one draws as it always
did.
_Avoid_: commit message (the summary is its body, not the whole), description,
gate summary (the gate is gone), changelog entry

**Notice**:
The one kind of Event Verkstead writes on its own account: which Stage it
started and where that Stage's branch went, that a roadmap has no Stage left to
run, or — as a **stop Notice** — what stopped driving, why, and what the
evidence was. No agent wrote it and nobody pressed anything for it. It is what
running unattended owes the human: a decision made while nobody was watching is
one they have to be able to read afterwards.

Nothing to do about one, however much it says. A Notice is written after the
fact and stays on the record for ever; what a stopped run is waiting on is that
it is **Stopped**, and what answers that is **Resume**.
_Avoid_: log line, info event, message, alert

**Transcript**:
The session's own record of its conversation — the agent's prose, its tool
use, its reasoning, and what was put to it — kept word for word as the agent's
backend wrote it, and rendered readable in the details pane. What summaries and a
stop Notice's evidence draw from, falling back to the Capture when a session
left none.
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
read-only.

Watching one commits nobody to anything, and neither does typing into one:
keystrokes reach the session and nothing else follows them — no register, no
badge, no Event, and nothing held off. Somebody who means to take the work on by
hand presses **Stop** first, and the Conversation being **Stopped** is what
holds the run off while they do; a session typed into while a run is still
driving it is ended and advanced by the ordinary rules.
_Avoid_: terminal, console, attach view, pane

**Agent Profile**:
A named coding-agent account Verkstead can run a session under: a claude home
directory and config file pair, the models that account can run, and an
agent-type discriminator so other backends can slot in later. The pair is
bind-mounted at `~/.claude` / `~/.claude.json` inside the sandbox, which is what
keeps accounts separate. The models are a list and the list is the Profile's
own, because different Profiles reach different accounts and each can launch
different things; none of them is a default, so which one a session runs is
always picked — as a Pairing, alongside the Profile itself.
_Avoid_: account, identity, persona, agent config

**Pairing**:
An Agent Profile and one of the models it lists, chosen together, and what a
session is actually launched under. Neither half runs anything alone: the
Profile says which account, its list says what that account can launch, and a
session runs one model. Offered as one flat list wherever it is picked — a row
per Profile-and-model combination — because the counts are small and a
two-stage pick would cost a tap every time. There is no default model
anywhere, so an unpaired Profile is half a choice and reads as none.
_Avoid_: profile choice, model selection, profile+model, combination

**Grilling Pairing** / **Implementation Pairing**:
The two Pairings a Conversation fixes before grilling starts — one for the
grilling session, one for the implementation work. They are roles a Pairing is
used in, not kinds of Pairing: the same Profile, even the same model, may fill
both. The line between them is planning against building: the grilling
session's tail — writing the handoff, the backlog or the roadmap — is the
Grilling Pairing's, and the Implementation Pairing drives what builds. Distinct
accounts are why an inline implementation is a fresh session rather than the
grilling session carrying on.

Both are **fixed when grilling starts**, alongside the branch, the base commit
and the Brief: what runs the work is settled before the work begins rather than
swapped underneath it — and the implementation one is used long after that,
which is exactly why it is not left changeable until then.

Each Repo **remembers the last pair it was grilled with**, so a new
Conversation on it arrives with both pickers already filled. Written at grill
start, from what the Conversation is actually running under; remembered
server-side rather than in a browser, because the workbench is answered from a
phone as readily as from a desk. A prefill and not a lock: it is the human's to
change before pressing, and what they changed it to is what gets remembered
next. A remembered Pairing whose Profile has broken, or which no longer lists
the model, is silently not applied — an unchosen picker, exactly as a Repo with
no memory gives.
_Avoid_: primary/secondary profile, planner/worker, grilling agent, grilling
profile (the Profile is half of it)

**Direction**:
How a Conversation's work gets built — **inline**, **task list** or **roadmap**
— picked by the human on a Proposal's own Set, never anywhere else. One of the
three and never a mixture: the choice is which pipeline runs the work, and a
Conversation that had picked two would be two pieces of work.

The pick informs the agent; artifacts move the machine. A pick is delivered to
the grilling session with the rest of the Response, and what proceeds from it
is that session producing the picked Direction's artifact — the handoff for
inline, the committed backlog for a task list, the committed roadmap for a
staged one. Verkstead moves on the artifact landing and the session going
quiet, never on the answer itself, so between pick and artifact the session may
come back with another Set instead, and a later Proposal's pick supersedes:
the latest pick is the one watched for. The answered Set is the record of the
choice — no Event of its own, and no state to sit in.
_Avoid_: mode, strategy, plan, execution path

**Proposal**:
The grilling agent's closing move: a recommended Direction and the reasoning
for it, carried as a block on a Question Set. What can end a grilling rather
than continue it — no button anywhere ends one. Ordinary grilling Sets carry
none, and at most one Proposal is ever in flight.

A Set carrying a Proposal is drawn with the Direction chooser on it: all three
Directions offered every time, the recommendation marked and never preselected,
the rationale beside them, and the chooser itself saying what picking does.
Picking a Direction accepts the Proposal. **Every other way of answering sends
it back**: an answer in the human's own words, questions left open, anything
without a pick. That is how the human disagrees, and it is the whole way back.

Accepting is soft either way: the whole Response returns to the session that
proposed, which judges for itself whether everything is clear — proceeding is
producing the picked Direction's artifact, and coming back is another Set,
with a fresh Proposal if it wants the Direction reconsidered. The Direction is
still never the agent's to change: it proceeds on the pick or argues by
proposing again.
_Avoid_: wrap-up request, handover, recommendation (that is the part, not the
whole), final question

**Handoff**:
The document a grilling session writes after an inline pick, and only then:
everything it settled with the human, written down for the fresh session that
builds the work. Inline's alone, because inline is the one Direction whose
builder crosses a context boundary — the two run under different Profiles, and
a session cannot change the account it is running as, so what the grilling
knows is written down or it is gone. A task list or roadmap needs none: the
committed backlog or roadmap is the plan, written by the context that settled
it. Written after the choice rather than before it, so a refused Proposal
costs no rewrite and the human's words beside the pick shape what is written.
The handoff is the inline tail's artifact: its presence, plus quiet, is what
ends the grilling session.

Verkstead's document rather than the project's, so it is written outside the
Worktree — in a directory of the Conversation's own under the Data Directory,
bound into every one of its Sandboxes. A handoff in the checkout would be a file
the next `git add -A` swept into the human's repository, and instructing an
agent not to commit something is worth less than the file not being there.

Taken onto the Timeline as an Event when the grilling session ends — the one
moment it is certainly finished — and taken rather than copied: the Timeline
holds the only one from then on.
_Avoid_: handover document, context dump, summary, notes

**Step**:
One piece of unattended work a session is launched for and ended after: a task
of a Conversation's backlog, the finish that follows the last task, an inline
implementation, which is the whole of the work in one Step, or the staging a
**Resume** launches — the breakdown, and the staging that ran the ordinary way,
are the grilling session's own tail rather than a Step. What is next is read
from the Repo and nowhere else — the lowest-numbered task file left in
`.tasks/`, or `TODO.md` on its own — so the Steps are the backlog's, and
Verkstead keeps no list of its own to disagree with it.

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
Pairings, primed with the stage brief as its Brief, and Implementing from the
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
write: what they settle is the two Pairings and the base commit, and the stage
brief becomes the Brief. One Stage is the whole of what adopting starts, and all
it has to start — that Stage's own plan commit writes to the roadmap, so when it
settles the Stage after it begins the ordinary unattended way, and an adopted
roadmap is a staged one from there on. Never stacks: there is no predecessor
Conversation to stack on, and building on an unmerged branch is the human's
move, made by picking that branch as the base.
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

**Stopped**:
What a Conversation is when nothing is driving it and nothing will start again
until somebody presses. The state, where **Stop** is a button and only one of
the ways a Conversation reaches it. Durable on the Conversation rather than
something that happened — what happened is the **stop Notice** beside it, which
says what stopped, why, and what the evidence was. At most one per
Conversation, however the run stopped: a Conversation that is stopped is stopped
once, and a second stop raised against one already stopped is the same stop
noticed twice.

Nothing advances past one. Every launch asks first, so a Conversation waiting
on **Resume** never quietly gets another agent spent on it, and its card carries
*blocked on you* for as long as it is stopped. Nothing about being stopped
reverts, resets or stashes anything: the repository is left exactly as the
session left it, which is what makes taking the Worktree on by hand possible at
all.

**Deliberate** or by **circumstance**, which is the one thing a restart has to
know. Verkstead pulling the brake — a session that fell over, checks that would
not go green, a finish Step that left no pull request, an Agent Profile out of
usage window — and the human pressing **Stop** are both deliberate, and a
restarting server leaves them alone. A driver a restart or a crash took away is
circumstance: nobody decided anything, so the next server up carries the work on
unasked. A deliberate stop Verkstead decided on is also pushed to the human's
devices; one nobody chose is not, a restart being free to pick it up, and
neither is the human's own Stop, they being the one person a notification would
be telling their own news.

**No stop resumes itself**, the usage-window one included. A run whose Agent
Profile has exhausted its window stops the way everything else does, and all
that tells it apart is what it carries: the Profile that ran out, and — where
the sentence the session printed carried a time this build could read — when the
window comes back, as words to show beside the **Resume**. Information rather
than a timer: nothing counts down to it, and nothing starts when it passes.
Recognition is one phrase read off the Capture and the Transcript, kept in one
place because the wording is the backend's and will move. The session is ended
along with the stop, the agent's own wait for the same reset being no reason to
have work going on inside a Conversation that reads as stopped. And **no
auto-switching between Profiles**: an exhausted account is a wait, never a
reason to spend a different one.
_Avoid_: halt and pause (the two names this had before there was one of it),
hold (gone, and nothing replaced it), interruption, error, failure, crash,
incident, alert, block, rate limit, throttle

**Resume**:
The one way a stopped Conversation gets going again, standing in the start-work
menu wherever there is driving to start. It recomputes what *ought* to be
running now — from the lifecycle the Conversation is in and what its branch has
written — and starts that, rather than running again whatever it was that
stopped. A stop may be answered the next morning, and the Conversation moves on
in the meantime; where the stop carries words about a usage window coming back,
they stand beside the button as text, and nothing is waiting on them.

It carries nothing. Steering the work is what **Steer** is for, so there is no
note to write and one button rather than one per way of stopping. It is never
silent either: either something starts, which the Timeline says by itself, or
the press is refused by name and the page says which — the backlog that has
gone, the Pairing that has, the Worktree that is nowhere. A Worktree the record
names and git does not is made again from the branch rather than refused on.

Offered on a Conversation that is merely undriven as much as on one that is
**Stopped**, a run with nothing behind it being the same condition however it
got there. **And a restart presses it for itself**, on every Conversation it was
left driving — except the stops somebody decided on, which are the ones waiting
for a person.

Beside it in the Conversation's own menu, the two presses that stop: **Stop**,
which lets whatever is running now reach its own end and stops before the next
launch, and **Force stop**, which ends the session where it stands.
_Avoid_: retry, remedy, restart (that is the server's), continue, unblock

**Steer**:
The human saying where the work goes, from wherever it has got to: a row in the
Conversation's own menu beside **Stop**, and a modal over one question — where
does this go? Targets are **Grilling**, **Implementing**, **Wrapping** and
**Done**, the four states the work is done in; Draft and Aborted are not among
them, each having a way in of its own. Sources are every state there is — a
Draft nothing has run in, a run in flight, work Verkstead has finished with —
because a steer is the human stepping outside the pipeline's path rather than
another move along it. So every refusal is about the target instead of the
source: wrapping up is offered only where the work is on a pull request, there
being no wrap-up to steer into otherwise.

**The click stops the drive**, before the modal opens. The ordinary Stop, so
nothing new launches while the human composes and whatever is running is left
exactly where it is. **Cancel leaves the Conversation stopped**, with **Resume**
on offer: the click froze the world, and unfreezing is a press of its own rather
than something a dismissed modal does behind the human's back.

**What ends the session running is the submit rather than the click.** One
Worktree holds one agent, so the session a steer starts takes the Worktree from
whatever is still in it — at once, or once a session that cannot be displaced
has finished, which is a review waiting on an Ask or a **Manual Task**. Into
**Done** nothing is started, so nothing takes it and the session runs to its own
end. **Interrupt current task** ends it where it stands instead, leaving the
step however far it had got: the wait saved in the first case, and the only
ending there is in the second.

**What is missing is made again**, and the further from a running state the
source is the more of it there is to make. A Worktree whose directory has gone
is checked out afresh from the branch, exactly as a pressed Resume makes one; a
Draft has no branch either, so it is cut where a grill start would have cut it —
off the base the human fixed, resolved at that moment.

**What a target takes is what it has to be about.** Grilling takes a new Brief,
optional, empty being the round starting on the one already there; and a choice
about priming the session with the digest of everything already answered, off
unless asked for, because a steer is usually a change of direction rather than a
return to the argument just left. Implementing takes a hand-written
instruction — required where the branch holds nothing to carry on, and optional
where it does, empty there meaning carry it on. Wrapping takes nothing: its
watchers recompute over whatever the branch now holds, the fix attempts
forgotten. Done takes nothing at all, there being nothing to run in it.

**The record is the move with the human's own line above it.** The Steer is an
Event of its own — somebody decided this — carrying the brief or the instruction
as its body, and the machine's plain Moved line stands under it. A steer into
Grilling lands that round's Brief under the move as well, frozen where it lands
and beside the earlier round's rather than over it. The Pairing the modal
settled is recorded as the **Conversation's** rather than one session's, because
steering re-settles what runs the work — which is also why the pick is part of
the form: a steered Draft has none fixed yet.

**And the submit resumes in the same press.** The stop the click left is
cleared, and what that state ought to be running starts — a fresh grilling, the
instruction session or the next step off the branch, the wrap-up's watchers.
Into Done it is the move alone.
_Avoid_: redirect, retarget, override, transition, take over, manual task (the
errand beside the work, not a way of moving it)

**Manual Task**:
A free-text instruction the human types at the end of a Conversation's Timeline,
with an Agent Profile picked beside it: submitting starts a one-off session that
does what it says and stops. Offered wherever the Conversation has a Worktree
and no session is registered for it — every quiet moment, in a driven state or
out of one. The escape hatch: the way to get work moving by hand when the
pipeline is not driving it.

Outside the pipeline in every sense that matters. It moves the Conversation into
no state and out of none, clears no stop, and reopens nothing a Done
Conversation settled; what it leaves behind is its instruction on the record,
what its session printed, and whatever that committed. The Pairing picked is for
that submission alone and never becomes the Conversation's — the composer starts
on the Conversation's implementation Pairing and otherwise asks for a pick.

Ended on quiet with no Question Set of its own still open, there being no done
file to end it by — so a session idling on a Blocking Ask is left where it is.

One whose session exits badly leaves the Conversation **Stopped**, with the stop
Notice saying so: the human submits from a phone and walks away, so being told
is the only thing that reaches them. A finished one leaves the Conversation
stopped either way — nothing takes the pipeline up again on the strength of a
Manual Task, and what does is **Resume**. One that exits cleanly having
committed nothing stops it too, an instruction that legitimately changed nothing
being indistinguishable from one that could not.
_Avoid_: step (the unattended unit a done file ends), task (a backlog's), take
over, errand, manual step

**Stalled**:
A Conversation in a driven state — Grilling, Implementing or Wrapping — with
nothing registered as driving it and nothing on it saying it is **Stopped**.
Nothing is moving the work and nothing is saying so, which is the one condition
Verkstead has to notice on its own account. A condition an active state can be in
rather than a state of its own — the Conversation is still Grilling or
Implementing or Wrapping, and that is the half of it that is wrong.

The condition rather than the record of it. A sweep looks every minute and
**stops** what it finds, by circumstance rather than by anybody's decision, so
what the human sees is a Conversation stopped with a Notice on it and a Resume
to press — and what a restart sees is one it may take up unasked.

Judged by whether a driver — a grilling session, the watcher a Pick armed on
one, a runner loop, a wrap-up's watchers — is registered, rather than by how
long nothing has happened. Wrapping idles for days under live watchers and is
perfectly healthy; so are the gaps between an unattended run's Steps. Draft,
Done and Aborted are never stalled, nothing being supposed to drive them.
_Avoid_: blocked on you (the badge a stall is precisely without), stopped (what
a stall becomes, not what it is), state, stuck, hung, idle

**Blocking Ask** / **Deferred Ask**:
The two ways an agent puts a Question Set to the human. A **Blocking Ask** idles
the session until the Response arrives, as every ask does in askance. A
**Deferred Ask** does not idle it: the Set waits in the Timeline and its
Answers are folded into a later session's prompt. Work blocks only on Questions
whose Answers affect work about to be done.

`verkstead ask --deferred` is the second one, and the difference is the session's
alone. Both land on the Timeline, both leave the Conversation *blocked on you*
and both notify the human's devices; a deferred one says on the Timeline that it
is deferred, and its badge says no agent is waiting rather than that one has
disconnected. What is deferred is how it was asked rather than anything in the
Set, so it is kept beside the stored body rather than in it.

The **folding** is the far end: when a session is started to build, every
answered Deferred Ask of that Conversation nobody has been told about goes into
its prompt, oldest first, under the documents the prompt is built from. Each is
folded once, and that it was folded is recorded rather than worked out from what
is answered. A Manual Task's session is never folded into — its prompt is the
instruction and nothing else — and neither is a relaunched grilling, which is
already primed with everything the Conversation has answered.
_Avoid_: sync/async ask, hard/soft question, urgent question

## Question Sets

**Question Set**:
A batch of Questions submitted together by one agent, with a Preface and a
title. The unit that lands on a Conversation's Timeline, gets answered, and is
archived. Reached through the Conversation it was asked from and nowhere else:
a second way in would be a second thing to keep true.
_Avoid_: request, batch, ticket

**Unreadable**:
What a stored Question Set is when the build looking at it cannot deserialize
the body it was written as — ordinary schema movement, a field having left. It
is drawn as a row saying so, on the Timeline it has always been on, with the
stored body reachable and nothing offered to answer or archive it by. The rule
is ADR-0006's, applied to the Sets themselves: keep what was written and defer
rendering it, so that one record the schema has outrun costs its own row and
never the Timeline around it. Nothing rewrites a body to make it readable —
it is the record of what was asked, and a later Verkstead should find it as it
was written. Distinct from Unanswered, which is a Question the human left open.
_Avoid_: corrupt, invalid, broken, unparseable, legacy

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
it, so an agent needs nothing beyond the binary to learn how to ask. A core
that every ask needs, plus any Topics fetched when their task arises — since
the gates Topic was retired the core is the whole of it.
_Avoid_: help, manual, docs

**Topic**:
A task-scoped section of the Guide, split out so an agent pays its reading
cost only when the task is at hand — at which point it is required reading,
never optional. There are none at present: the gates Topic was the only one,
and it went when the last gate did.
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
The data-free signal telling an open viewer page where the world moved — what
kind of thing changed, and which Conversation it belongs to where one does —
so the page re-reads only what it shows of that. It points and never carries:
what changed rides no Nudge, and the page fetches it the ordinary way. A Nudge
of a kind the page does not know, a reconnected stream, and a page returning
to visibility each fall back to re-reading everything — which is also the
whole meaning of the push-relayed Nudge. A query whose rendering holds reader
state must still reconcile its re-reads, or be `static` where its payload
cannot change (ADR-0005).
_Avoid_: tick, refresh signal, ping, change event, notification

**Update Notice**:
The banner the viewer shows when the server has learned that a newer release
exists than the one it is running, linking to the update instructions. Informs
only — nothing is installed on the human's behalf.
_Avoid_: upgrade prompt, new-version alert
