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
**Registering one can be taken back, and that is an unregistering rather than a
delete**: Verkstead stops offering it for new work — the settings list, the New
conversation menu, the roadmaps waiting to be adopted — while every Conversation
ever worked in it goes on naming it, because a record that could not say which
repository its work was done in would be no record. The directory is untouched
either way. Refused while a Conversation that is neither Done nor Closed is on
it, the way removing an Agent Profile a Conversation is set to run under is; and
registering the same path again brings the same Repo back rather than making a
second one.
_Avoid_: project, codebase, checkout

**Conversation**:
The core entity: a Repo, a base commit, a Brief, one branch and one Worktree.
Everything done about one piece of work hangs off it. Runs through Draft →
Grilling → Implementing → Wrapping → Done — a roadmap Conversation passes
straight from Grilling to Wrapping, its building belonging to its Stages — and
can be closed from any of them, a **Steer** being the one way back into a
Conversation that is closed or Done. There is one move back down the ladder: a
wrap-up whose review split its findings out into a backlog returns to
Implementing to build it, and its finish step wraps up again on the pull request
it already had, reviewed afresh. **Follow-up** sits beside the ladder rather
than on it, the way Closed does, and is the one state with no way in but a
Steer: the human taking something up about work that is already on a pull
request, and landing back in the wrap-up when they are finished with it.
*Blocked on you* is a badge on an active state, never a state of its own, and
*Waiting on checks* is a condition of Wrapping read the same way — where
**Closed** is a state of its own, off the ladder rather than on it: every other
state is somewhere the work has got to, and closing is the work stopping
wherever it was. Nothing about a Closed Conversation waits on the human:
closing shuts every Question Set it left open — those read *closed unanswered*
on the Timeline — and neither waiting mark is drawn over the stop it carries,
which stays on the record as history. Done is not Closed in this. A Done
Conversation's Sets are still there to be answered, so one left open goes on
drawing the marks: an answerable ask is still an ask.

**What it is called is its branch, where anybody has named one.** A Conversation
is started on a name Verkstead invented, because there has to be a branch to cut
and nobody has thought about the work yet — and a name nobody chose says nothing
about the work, so while it is a Draft none of it is drawn anywhere: the sidebar
row, the pane header and the row read aloud all call it **Draft**, and the
branch field on the setup card stands empty under *Automatically select*. Whose
the name is is kept in the record rather than read off the name's shape. Typing
one settles it, and it is the title from that moment; clearing the field hands
the naming back, and the name the Conversation started on stands again rather
than another being invented. Two drafts against one Repo both reading *Draft*
beside the same Repo name is what two drafts are: they are few, and they are
short-lived.

**And where nobody has named it, the work's first session is asked to.** The
press that starts the work leaves the naming to the session it starts — the
grilling one, the ungrilled build one, or the one a steered Draft starts — and
its prompt carries the instruction under the Brief: switch the branch to a short
kebab-case name taken from what the work is about, with git, before anything
lands on it. Nothing is asked back, a rename being read off the checkout the way
commits are. A Conversation the human named carries no such instruction, having
nothing to leave to anybody.

Which is why the **Draft** title outlives the Draft. Starting the work is not
what makes an invented name worth reading, so the sidebar row, the pane header
and the row read aloud go on saying *Draft* through the first minutes of
Grilling or Implementing — and say the branch the moment the name is settled.
Two things settle it and both are final: the session renames the branch and
Verkstead follows it, or the session ends having left the name alone, and the
name it left is the Conversation's. The setup card is not part of this: the
branch is a plan while the Conversation drafts and a fact from the moment it is
cut, so the field goes when the card does, whatever the name on it turns out to
be.
_Avoid_: task, session, job, thread, ticket

**Worktree**:
The checkout a Conversation's work is done in, made when grilling starts along
with the branch it holds, and removed when the Conversation is closed — the
branch outlives it, because a branch is cheap and may hold work worth reading.
A Conversation may have more than one: its own, and one per Companion Repo,
made when its own is made and given back when its own is. A steered
Conversation keeps the one it has; where the directory has gone, one is checked
out again on the branch that was worked, which is one of the two times a
Worktree is made without a branch being made with it — a read-only companion's,
checked out detached, is the other.
A removal git refuses — a directory it no longer reads as a Worktree — does not
hold the close up: it is logged with its path and left on disk, closing being
what the human asked for and a directory nobody can be rid of being what they
were trying to escape.
Named for the Repo and what the checkout holds — the branch, or the base a
detached one stands at — and it lives in the Data Directory rather than inside
a Watched Path: Verkstead made it, so it goes among Verkstead's own things.
**A session may rename the branch in its Worktree, and Verkstead follows it.**
A recorded branch that is gone from the Repo while the checkout sits on another
branch is a rename: the record moves to the new name, and every mirroring
Companion Repo's branch is renamed to match in the same act. A recorded branch
still standing while the checkout is elsewhere, or a checkout on a detached
HEAD, is a Worktree that has come adrift and rebuilds as it always did. The
directory keeps the name it was made with either way — it is cosmetic, and
moving a live Worktree is another way to fail.
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
Profile's pair at `~/.claude` and `~/.claude.json`, the system, the Skills at
`/verkstead/skills` and the Verkstead executable read-only, and nothing else of
the machine at all — not even the checkout the Worktree was made from. An empty
directory read-only over `~/.claude/skills` goes with the Skills' own mount:
the account's are hidden rather than merged with, and a mount at a path no
backend owns hides nothing by itself. Each Companion Repo the
Conversation was configured with is inside as well: its Worktree and the git
directory behind it, both at that companion's own mode, so a read-only one is
read-only through both. The **Build Cache** is inside as well, writable, with
the `sccache` it compiles through read-only beside the executable — that one is
a client, and what it reaches is the **Compile Server** in a Sandbox of
Verkstead's own. The filesystem is the boundary and the network is not:
inside, it is the host's own, whole and unfiltered, because what stops a
session doing harm is that there is nothing within reach to harm. The
`verkstead` a session asks with is the running server's own image, first on the
`PATH` inside, so the CLI a session asks with and the server it asks are one
build and cannot disagree about a schema.
_Avoid_: container, jail, isolation, environment

**Sandbox Configuration**:
The extra writable binds a Sandbox gets beyond that surface — a package
registry's, a cache Verkstead does not provide — as one global set every Sandbox
gets plus a per-Repo set composed over it. Configured where the Watched Paths
are rather than anywhere the workbench can reach: every one of them is a
directory of somebody else's and a hole in the boundary, and widening a boundary
is the installer's to do. The **Build Cache** is not one of these and is not
configured here.
_Avoid_: sandbox settings, mounts, extra paths

**Build Cache**:
One directory of Verkstead's own that every Sandbox gets writable, so a Rust
dependency is downloaded and compiled once for the machine rather than once per
Conversation. The server's own feature rather than Sandbox Configuration: it
makes the directory, it resolves the `sccache` it compiles through off its own
environment, and it is **on with nothing configured** — a human should never
have a worse experience for not having checked the settings. Where it is is the
installer's (`--build-cache-dir`, else the XDG cache directory; the packaged
unit says `/var/cache/verkstead`); whether a Sandbox gets one at all, and how
big its compiled half may grow, is the human's, in the workbench settings. The
one control there that reaches inside a Sandbox, and it only ever closes the
hole. Without an sccache it is still a cache — the crate downloads are shared —
and the setup card says so on a repository that builds Rust.
_Avoid_: sccache, cargo cache, artifact cache, shared target dir

**Compile Server**:
The one `sccache` server the machine compiles through, run by Verkstead in a
**Sandbox of its own**. An sccache server is what actually executes `rustc`, and
every Sandbox shares the host's network — so sessions left to start their own
all reach for one port, and whichever lost the race has its compiles run inside
another session's Sandbox, where its Worktree is not bound and the build fails.
Started before the first session of a Conversation whose Repo builds Rust, and
never on a machine that builds none. Its Sandbox holds the Worktrees directory —
all of it, so a Conversation grilled later is one it can already compile for —
and the Build Cache, and nothing else Verkstead keeps: `rustc` runs proc macros
while it compiles, so the database and the settings files stay outside its
reach.
_Avoid_: daemon, sccache daemon, build server, compiler service

**Companion Repo**:
Another registered Repo a Conversation is given to work alongside its own,
**read-only** or **read-write**, checked out beside the Conversation's own
Worktree and bound into every one of its sessions' Sandboxes at whichever of
the two it is set to. Registered is the whole of the trust boundary: what a
Conversation may compose is what the human has already put in the registry. The
Conversation's own Repo is refused — it is the work's repository already, and a
second checkout of it in one sandbox is not a companion — and so is a Repo
added twice.

**Configured beside the branch while the Brief drafts**, on the setup card and
by the setup's own rules: freely added, edited and taken away, and frozen at
grill start along with the branch and the base. What is configured is the mode,
the branch the checkout comes off — that repository's default branch where none
is picked, the rule the Conversation's own base follows — and, for a read-write
one, what its branch is to be called. Empty is not a branch called nothing but
*mirroring*: the Conversation's own branch name, followed as it is renamed,
until a name is typed and stands on its own.

**Frozen only means it stops narrowing.** A **Steer** may put another
registered Repo in and open a read-only one up to read-write, on every target
the work goes on in, because what that settles is the sandbox the sessions to
come run in rather than a property of one state. Never the other way: nothing
removes a companion and nothing puts one back to read-only, so what a session
was once given is never taken back mid-Conversation. What is added at a steer
answers the setup card's questions again, and what is opened up is cut its
branch off the base its row already names, re-resolved at that moment — the
repository is joining the work now, so it starts from now rather than from the
commit its detached checkout was left at.

**A Stage inherits its predecessor's whole set**, through the one act that
gives it the Pairings and the stage brief: a stage has no draft moment of its
own, so there is nowhere else the set could come from, and a roadmap grilled
against a repository would otherwise build without it. Read-only ones come
across as they are; read-write ones cut a branch of their own per stage, named
after the stage's own branch rather than carrying a name somebody typed while
drafting the roadmap, because two stages sharing one companion branch would be
two review units on one branch. Where the stage's own branch stacks on its
predecessor's, its companion branches stack too.

**Always a Worktree of Verkstead's own, never the human's checkout.** Whenever
one is made — a grill start, an adopted stage, a steer, a stage a settling
predecessor starts — each companion is fetched and then resolved, the
Conversation's repository's order for the Conversation's repository's reasons.
A read-only one is checked out **detached** at the commit its base resolved to,
having nothing to commit and no business taking a name in somebody else's
repository; a read-write one is cut a branch from its base, exactly as the
Conversation's repository is. Every question is asked before anything is made,
so an answer git will not give — the fetch that failed, the base that resolves
to nothing, the branch already taken — refuses the whole press naming the
repository and leaves neither a directory nor a branch behind; where nobody is
at a button, it halts the stage and says so on the Timeline instead. Closed the
way the Conversation's own is: the directory goes and the branch stays.

**A session gets the checkout, and is told it is there.** The worktree and the
git directory behind it are both bound at the companion's mode — read-only
reaches the git directory too, or the history would be writable around the back
— and the companion Repo's own Sandbox Configuration binds are composed in
beside them, because building in it needs them. Its flake is not entered for
the session: the dev shell is the Conversation's own worktree's alone, and an
agent that needs a companion's enters it itself, `nix` being on the sandbox
`PATH`. The prompt carries one neutral `# Companion repositories` listing —
each one named with its directory, what it holds and whether it may be written
to — and no instructions with it, because what a companion is for is the
Brief's to say. What a Conversation was configured with is read on the Brief's
details pane ever after, the setup rows having gone when the card froze.

**A read-write one's branch is swept like the Conversation's own.** Every commit
that lands on it reaches the same Timeline, labelled with the Repo's registered
name — where a commit in the Conversation's own repository draws unlabelled,
because an unlabelled card means the work's own repo and the label earns its
place when repos mix. Opening one shows its diff read out of the companion's
repository. A read-only companion is swept by nothing: its checkout is detached
and bound read-only, so there is nothing there for a commit to land on.
_Avoid_: submodule, dependency, linked repo, sibling checkout, secondary repo

**Skill**:
One of the workflows Verkstead runs its sessions by — grilling, implementing,
breaking down, working a Step and following up now, the rest as the stages that
need them arrive. Verkstead's own: shipped inside the binary, installed under
the Data Directory at startup and mounted read-only at `/verkstead/skills` — a
path no backend owns, beside the Verkstead executable's — so a session's
behaviour is the product's rather than whatever the machine or the account
happens to keep. What the account keeps is hidden by an empty directory bound
over `~/.claude/skills`, which is the mounting the Skills used to do there. A
session is put inside one by the prompt it is started on, which names the Skill
above the Brief.
_Avoid_: prompt, instructions, plugin, workflow file

**Brief**:
The editable markdown document a round of a Conversation starts from, and the
first Timeline Event. Freezes when its round's grilling starts; a steer into
Grilling opens a round with a new Brief rather than editing the frozen one, so a
Conversation has one Brief per round and the newest is the one being written.
What is *not* the human's again on a later round is the branch and the base
commit: the branch has been worked, and the second round carries on from what is
on it.

Written where it is read: while its round is drafting, the Brief on its card
*is* the field — raw markdown, always open, keeping itself whenever the typing
stops for a moment and whenever the field is left, and saying nothing about it
either way.
There is no Edit, no Save and no word about saving, because there is no other
thing the Brief could be doing while it is a draft. Once it freezes it is the
server's rendering of it and nothing else.

While it is still a draft its card carries the whole of the Conversation's
setup under it — the branch, the base commit, the Pairings and the readiness
verdict — because setting the work up and kicking it off are one act, and both
belong where the work is read. Every one of those freezes at the same moment
the Brief does, so once grilling starts the card is the Brief alone; on a
later round the branch and the base commit are frozen already, and what the
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
The agent-written account a code commit carries as its message body — prose
first, a delta Diagram after it — kept by the sweep of whichever repository it
landed in with its trailers stripped,
rendered as the **Message** above the diff in the commit's details pane —
headed and boxed there the way a Set's Preface is — and clamped to a prose
snippet on its Timeline card. Written for commits that deliver work; pure
bookkeeping commits carry none, and a commit without one draws as it always
did.
_Avoid_: commit message (the summary is its body, not the whole), description,
gate summary (the gate is gone), changelog entry

**Notice**:
The one kind of Event Verkstead writes on its own account: which Stage it
started and where that Stage's branch went, that a roadmap has no Stage left to
run, that a wrap-up is down to its checks, or — as a **stop Notice** — what
stopped driving, why, and what the evidence was. No agent wrote it and nobody
pressed anything for it. It is what running unattended owes the human: a
decision made while nobody was watching is one they have to be able to read
afterwards.

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
A named coding-agent account Verkstead can run a session under: an agent type,
the account itself, and the models that account can run. **The account's shape
is its type's**, rather than one shape every Profile is assumed to have — Claude
Code's is the directory and config file pair bind-mounted at `~/.claude` /
`~/.claude.json` inside the sandbox, and every backend after it keeps its whole
account under one relocatable home — Codex's at `~/.codex`. Whichever it is,
mounting it is what keeps accounts separate. A type is offered to the human only
once it can launch the real thing: one that cannot would be a lie in a picker,
so the form still writes Claude alone and a Profile of a later type is one saved
over the API until its stage lands. The models are a list and the list is the
Profile's own, because
different Profiles reach different accounts and each can launch different
things; none of them is a default, so which one a session runs is always picked
— as a Pairing, alongside the Profile itself.
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

**Grilling Pairing** / **Implementation Pairing** / **Review Pairing**:
The three Pairings a Conversation fixes before grilling starts — one for the
grilling session, one for the implementation work, one for the wrap-up's review.
They are roles a Pairing is used in, not kinds of Pairing: the same Profile,
even the same model, may fill all three. The first line between them is planning
against building: the grilling session's tail — writing the handoff, the backlog
or the roadmap — is the Grilling Pairing's, and the Implementation Pairing
drives what builds. Distinct accounts are why an inline implementation is a
fresh session rather than the grilling session carrying on.

The second line is reviewing against fixing. The Review Pairing reaches exactly
one kind of session — the wrap-up's review, including the fresh one that runs
after a split-out backlog is built — because reviewing is a fresh set of eyes on
what was built. Every other session a wrap-up dispatches is the work itself
carrying on and runs under the Implementation Pairing: the check fixes, the
comment responses, the follow-ups, and the session sent for a missing pull
request.

**A review with no Pairing of its own runs under the Implementation Pairing**,
which is the one place a role reaches past itself for an account. Not a default
— the picker is answered before the work starts, and a Conversation started
since has one — but the two ways of arriving at a wrap-up with the role never
picked: a Conversation from before the role existed, whose pickers froze when
its work started, and a Draft steered into a state that settles only what
builds. Both were reviewed by whatever built them before there was a Review
Pairing, and a wrap-up that would not review them at all is one waiting for ever
on a review nothing can start. **No review** is not this and never falls back:
it settles the review rather than leaving it unpicked, which is the whole
difference between the row and an empty picker.

Two of the pickers offer a row that is not an account: **No grilling** and **No
review**. Picking one is a choice like picking a Pairing — it satisfies
readiness, freezes when the work starts, is remembered per Repo and inherited by
a stage — and what it settles is that the role runs no session at all. An empty
picker is not that: the two leave the same thing unchosen and only one of them
lets the work start.

**No grilling** takes the Brief straight to the work. The one press does
everything a grill start does — fetch, resolve the base, cut the branch, make
the Worktree and every companion's, freeze the Brief and the Pairings — and
lands the Conversation Implementing rather than Grilling, with an inline session
under the Implementation Pairing primed on the Brief alone. Its prompt says
there was no interview, so a real decision the Brief leaves open is put to the
human as a Blocking Ask rather than guessed at. Inline only — no backlog and no
roadmap — and watched out to a pull request and an ordinary wrap-up exactly as
an inline implementation picked at the end of a grilling is. The button reads
**Start work** whichever way the Conversation starts.

**No review** settles the wrap-up's review the moment it looks, with no
session and nothing on the Timeline, and everything else runs exactly as it
always does: the checks with their two fix attempts apiece, what is said on the
pull request answered in batches, Done once the suites are green. With no review
there is nothing to split findings out of, so the one path back down the ladder
never opens from a wrap-up like this.

Every one of them is **fixed when grilling starts**, alongside the branch, the
base commit and the Brief: what runs the work is settled before the work begins
rather than swapped underneath it — and the implementation and review ones are
used long after that, which is exactly why they are not left changeable until
then.

Each Repo **remembers the last set it was grilled with**, so a new
Conversation on it arrives with every picker already filled. Written at grill
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
from the Repo and nowhere else — the lowest-numbered entry of `.tasks/TODO.md`
whose box is not ticked, or the finish once every box is — so the Steps are the
backlog's, and Verkstead keeps no list of its own to disagree with it. An
unticked entry naming a task file nobody wrote is neither: the run stops there
rather than putting a session at nothing to work from.

A Step is **done** when what it turns on has changed in the Worktree *and* the
commit saying so has landed — a task's entry ticked off in `TODO.md`, the finish
step's `.tasks/` taken away. A session reports through the repository, being an
ordinary interactive one, and a commit is the one report it cannot half make — a
box ticked but not committed is a session still mid-Step.

Its session is ended once the Step is done **and** the session has gone quiet for
a grace period, never on done alone: work does not always stop at the commit, and
output arriving puts the whole grace back on the clock. A session that keeps
talking is never ended. One Step per session and one session per Step — a fresh
context each time, which is what the backlog was broken into slices for.

**A Step can be done and still be short**, because what a run's last Step is for
happens after its commit: the finish takes the list away and commits it, and the
push and the pull request come next — as they do for an inline implementation and
for a roadmap's own session, every kind of work here ending on a pull request.
One that stopped in between leaves the work built, committed and unreviewable, so
the missing thing is asked for — a session of its own, sent to push and open the
pull request and told the work is already built. Not a Step itself: nothing in
the Repo says it is due and nothing there says it is done, GitHub being the one
that knows. Once per go, and then the ordinary stop where the answer has not
changed, because two agents that both stopped short of the same push is something
for the human to look at.

**Resume takes that same go**, so a run stopped at its push is one a press can
finish. What it must not do is guess: an empty `.tasks/` is a backlog worked
through or one that never landed, which are opposite situations, so the branch is
read for which it is — a branch that has written a backlog since it came off its
base has been worked and finished with, and one that has written none has nothing
built to carry anywhere and stops.
_Avoid_: job, iteration, unit of work, stage (that is a roadmap's)

**Stage**:
One numbered entry of a roadmap, and a Conversation of its own: one branch, one
review unit, one pull request. Started by the Stage before it settling rather
than by anybody pressing anything — against the same Repo, under the same
Pairings, primed with the stage brief as its Brief, and Implementing from the
first moment, because the grilling that would have settled the work wrote the
brief. Its branch stacks on the unmerged predecessor where the target
repository records how, and comes off the default branch where it does not.

**Its branch is named for where the stage lives**: the roadmap's own directory
name, then the stage brief's filename — `docs/roadmaps/mvp/04-wrap-up.md` is
worked on `mvp/04-wrap-up`. Under the roadmap rather than at the bare slug,
because a repository is full of branches somebody named for whatever they were
doing and one of them reading like a stage brief says nothing about that stage —
and a name already taken is one Verkstead will not start on, in a run nobody is
watching. Qualified this way the only thing it can collide with is another
attempt at the same Stage of the same roadmap, which is the collision the
refusal is for.

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
write: what they settle is the Pairings and the base commit, and the stage
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
still exists, and the Stage's own branch not taken. A roadmap that is
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
on **Resume** never quietly gets another agent spent on it. Nothing about being
stopped reverts, resets or stashes anything: the repository is left exactly as
the session left it, which is what makes taking the Worktree on by hand possible
at all.

Who stopped it is recorded, and two things follow from it. **Verkstead** is the
brake it pulled — a session that fell over, checks that would not go green, work
whose pull request never arrived even after a session was sent for it, an Agent
Profile out of usage window. **Human** is their press on **Stop** or **Force
stop**. **Circumstance** is a driver a restart or a crash took away, nobody
having decided anything. **Deliberate** is the fourth word and the only one
nothing writes any more: a stop recorded before the first two were told apart,
read as the human's, because their own presses are what nearly all of those rows
are.

*Is it waiting for a press?* Everything but circumstance is: the next server up
carries a circumstance stop on unasked, and leaves every other one exactly where
it stands. *Is the human being told?* Verkstead's brake and a stop nobody chose
carry the full marks — the sidebar's disc and the *blocked on you* badge — and a
stop the human made themselves carries neither, showing a quiet **Stopped**
label in the Conversation's header instead, which goes to the same stop Notice.
They were there when they pressed it, and a mark that appears where nothing
happened without them is the mark that teaches them to stop reading the marks.
Push follows the same rule for the same reason: Verkstead's brake reaches a
phone, a stop nobody chose does not — a restart being free to pick it up — and
neither does the human's own Stop.

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
names and git does not is made again from the branch rather than refused on. And
a **Stage** whose planning session died before it committed is started rather
than refused: the plan session is launched as the Stage is made and by nothing
else, so a Stage with no `.tasks/` is a run that never began rather than one
that is worked out — read off what its branch has written since it was made,
which the backlog of the Stage it stacks on is no part of.

**A press always has somewhere to go**, which is the whole point of recomputing
rather than repeating: every state the pipeline can stop in has a next move, and
work that stopped at its push has the plainest of them — the pull request is
sent for again. What the branch has written since it came off its base is what
says which situation an empty `.tasks/` is, the same reading a Stage's planning
turns on, and the one press that is still refused by name is the branch with
nothing built on it at all.

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
does this go? Targets are **Grilling**, **Implementing**, **Wrapping**,
**Follow-up** and **Done** — the four states the work is done in, and Follow-up
beside them because a steer is the only way into it at all; Draft and Closed are
not among them, each having a way in of its own. Sources are every state there
is — a Draft nothing has run in, a run in flight, work Verkstead has finished
with — because a steer is the human stepping outside the pipeline's path rather
than another move along it. So every refusal is about the target instead of the
source: wrapping up and following up are offered only where the work is on a
pull request, there being no wrap-up to steer into and nothing to follow up
otherwise. Follow-up is the one target the source narrows as well, and by what
the modal offers rather than by a refusal on arrival — from Done and from
Wrapping alone, because work still being built has the ordinary ways of saying
what to do next.

**The click stops the drive**, before the modal opens. The ordinary Stop, so
nothing new launches while the human composes and whatever is running is left
exactly where it is. **Cancel leaves the Conversation stopped**, with **Resume**
on offer: the click froze the world, and unfreezing is a press of its own rather
than something a dismissed modal does behind the human's back.

**What ends the session running is the submit rather than the click.** One
Worktree holds one agent, so the session a steer starts takes the Worktree from
whatever is still in it — at once, or once a session that cannot be displaced
has finished, which is a review waiting on an Ask. Into **Done** nothing is
started, so nothing takes it and the session runs to its own end. **Interrupt
current task** ends it where it stands instead, leaving the step however far it
had got: the wait saved in the first case, and the only ending there is in the
second.

**What is missing is made again**, and the further from a running state the
source is the more of it there is to make. A Worktree whose directory has gone
is checked out afresh from the branch, exactly as a pressed Resume makes one; a
Draft has no branch either, so it is cut where a grill start would have cut it —
off the base the human fixed, resolved at that moment. Every **Companion Repo**
the record holds with nothing on disk is checked out again beside it, which is
what a steered Draft and a Conversation steered back out of Closed both need:
without it either would reach a running state with companions the sandbox skips
in silence.

**What a target takes is what it has to be about.** Grilling takes a new Brief,
optional, empty being the round starting on the one already there; and a choice
about priming the session with the digest of everything already answered, off
unless asked for, because a steer is usually a change of direction rather than a
return to the argument just left. Implementing takes a hand-written
instruction — required where the branch holds nothing to carry on, and optional
where it does, empty there meaning carry it on. Follow-up takes the brief its
session is opened on, and takes it *always*: an empty instruction carries the
branch on and an empty brief grills the one already written, and there is
nothing on the branch a follow-up could fall back on — it is something the human
wanted rather than a step of the run. Wrapping takes nothing: its watchers
recompute over whatever the branch now holds, the fix attempts forgotten. Done
takes nothing at all, there being nothing to run in it.

**And every target work goes on in takes the sandbox.** A companion section
under all three of them — Grilling, Implementing and Wrapping — because what it
settles is the world the sessions to come run in rather than a payload of one
state: the **Companion Repos** already there to read, a tick that opens a
read-only one up to read-write, and a row per registered Repo not on the
Conversation yet, asking what a setup row asks. Not under Done, where nothing
runs and there is nothing a companion could be for. One direction only: no row
offers removal and no switch offers read-only, so a downgrade cannot be spelled
at all. Everything it takes in is checked out as the steer lands, and a
companion it cannot deliver refuses the whole press by name — a press that did
not happen, with no directory, no branch, no row, nothing ended and no stop
cleared.

**The record is the move with the human's own line above it.** The Steer is an
Event of its own — somebody decided this — carrying the instruction, or the
brief a follow-up is opened on, as its body, and the machine's plain Moved line
stands under it. A steer into Grilling lands that round's Brief under the move
as well, frozen where it lands and beside the earlier round's rather than over
it. The Pairing the modal settled is recorded as the **Conversation's** rather
than one session's, because steering re-settles what runs the work — which is
also why the pick is part of the form: a steered Draft has none fixed yet. It
settles the role the target runs its sessions under, which is one apiece except
for a wrap-up: that both builds and reviews, so the one pick reaches the Review
Pairing as well — but only to **fill** one nothing was picked for, never to
replace one that was. The picker is labelled for the state's own work and
prefilled with what builds, so a human who changes nothing on it has said
nothing about the review, and an account they chose on the setup card to be a
fresh set of eyes stays that.

**And the submit resumes in the same press.** The stop the click left is
cleared, and what that state ought to be running starts — a fresh grilling, the
instruction session or the next step off the branch, the wrap-up's watchers, the
follow-up's own session. Into Done it is the move alone.
_Avoid_: redirect, retarget, override, transition, take over, reopen and manual
task (both retired: a steer into Grilling is what reopening a Done Conversation
was, and a steer into Implementing with a hand-written instruction is what a
manual task was)

**Follow-up**:
Where a Conversation goes when the human wants something taken up about work
that is already on a pull request: a session of their own, answering what they
ask and doing what they want done about it, for as many rounds as they want.
Not a rung of the ladder — it hangs off the wrap-up rather than following it —
and the one state with no way in but a **Steer**, offered from Done and from
Wrapping and only where the record holds a pull request.

**The brief is what it is about**, written into the steer and required there:
nothing on the branch could stand in for it, a follow-up being something the
human wanted rather than a step of the run. It lands as the Steer Event's own
body, which is where a session started again reads it back from — along with
the rounds already answered under it, this follow-up's own rather than the
Conversation's, both read from the newest steer into Follow-up down.

**The rounds are ordinary Question Sets.** The session answers what was asked,
does what was asked for, commits and pushes it — the branch is on a pull
request already, so the checks run while the human reads — and puts the round
to them as one Set. Nothing about the state makes its Sets special: the agent
writes an ordinary Preface, ordinary Questions and an ordinary Postscript, and
it never asks whether there is anything else.

**What ends it is the human's mark and the session's silence together**: the
newest round they answered carries **Nothing else**, nothing is left open on
the Conversation, and the session has printed nothing for the grace. The mark
alone would end a follow-up in the middle of the work the last round asked for;
quiet alone would reap a session idling on a Blocking Ask, which is one doing
exactly what it should. Then the session is ended and the Conversation is
Wrapping again over the pull request it was opened about, with the checks put
back to waiting where the follow-up pushed anything — *back to Done* being that
wrap-up's own settling rule rather than anything a follow-up decides.
_Avoid_: follow-up task, comment round, reopen, chat, Q&A

**Nothing else**:
The control that ends a follow-up: a checkbox in the closing section of a
follow-up's Question Set, under the set-level comment box it shares that
section with, saying there is nothing more the human wants from this follow-up.
Drawn on a Follow-up's Sets and on no others.

Not a Question and not an Option. It answers nothing anybody asked, so it rides
the Response as a field of its own the way a picked Direction does; and it is a
checkbox rather than an Option because there is nothing here to choose between,
a second click taking it off again.

**The agent never sees it.** The mark comes off the Response on the way into
the store and is kept beside it, so what a waiting session is handed is byte
for byte what it would have been handed without one. The agent writes an
ordinary Postscript and reads an ordinary Response; how a follow-up ends is
Verkstead's business rather than the session's.

**Never sticky, and the newest answer is the one that decides.** Everything
written beside it still goes back and the agent finishes the round, so a round
that ticks it and asks for one more thing in the comment goes round again — and
an answer without the mark puts the follow-up back to running.
_Avoid_: anything-else question (the agent asks none), done, close, opt-out,
finish flag

**Waiting on checks**:
What a wrap-up has narrowed to when the review is answered, nothing said on the
pull request is left unaddressed, the checks alone are outstanding and nothing
is running in the Worktree. A **Notice** says so on the Timeline, once per
narrowing; the card carries the words as a label beside the branch, and the
sidebar row reads them in place of the state word, Wrapping being what has
narrowed.

A condition of Wrapping rather than a state, exactly as *blocked on you* is a
badge on an active state: the Lifecycle is untouched, and the only thing stored
is the mark saying the line has been written. What the label is drawn from is
the wrap-up's own settle facts and the sessions register, read together every
time they are asked.

Nothing to do about one, which is why it is drawn quietly and pushed to no
device: the checks are GitHub's to finish. Leaving the condition takes the mark
with it, so a fix session dispatched or a comment landing and the wrap-up
quietening again is a fresh Notice rather than a duplicate of the first or a
silence.
_Avoid_: blocked on you (that is about the human, this is about GitHub), CI
(the word here is checks), pending, green, state

**Check rollup**:
How a pull request's checks are getting on, taken all together and said in one
word: anything red reads as *failed*, else anything unfinished reads as
*running*, else *passed*. The **pull request** card draws it as the icon GitHub
draws beside a pull request — a tick, a cross, a dot — on the right of its head.

Written down on every poll of the checks watcher, so it outlives both the poll
and the server: a Conversation carried to Done keeps the icon the last poll
earned it. Which also means it can be stale, the watching stopping when the
wrap-up is over — and what freshens a stale one is opening the pull request's
details pane, which asks GitHub the same question on its way to listing every
check by name, with its own mark and a link to its run.

Never guessed at. A pull request nothing has asked GitHub about has no rollup,
and neither has one in a repository with no CI — *not known* is a third thing
beside green and red, exactly as it is for the watcher meeting a `gh` that will
not answer, and what a card with no rollup draws is no icon.
_Avoid_: status, CI (the word here is checks), green as a state name, one check
(the rollup is the whole suite)

**Rescue**:
The canned line Verkstead types into a session that has gone quiet without
asking anything or finishing what it was sent for. A session reaches the human
in exactly one way — the Question Set it sends — so one sitting there with its
turn over, nothing open on the Conversation and nothing to show for itself
leaves a Conversation nobody can move. So it is spoken to, through the terminal
a watcher's keystrokes go through, and told the one thing it cannot see from
inside: that nothing it prints reaches anybody, and that a Set is the whole of
how the human is spoken to.

**Three things at once**, and none of them is enough alone: idle for the grace,
nothing open on the Conversation, and nothing landed. A session still printing
is at work, one sitting on a Blocking Ask is waiting on the human — for as long
as they take — and one that has landed what it was sent for is already being
ended by the driver beside this. An answer arriving, or a line typed in, starts
the grace again.

**Every session Verkstead launches**, one loop with the state's own
done-indicator as its parameter: a grilling's artifact, a backlog Step's task
file, an inline implementation's, an instruction's or a fix's commit, a
follow-up's Nothing-else mark. What differs from one state to the next is only
what *finished* looks like.

**Twice at most**, the second silence being evidence rather than bad luck. A
session still saying nothing after the second is ended where it stands and the
Conversation **Stopped**, with a Notice saying it would not ask and **Resume**
for the human to press — except a fix session, which is ended and its check
looked at again, the wrap-up's two goes at a check being the stop it already
has. Nothing goes on the Timeline for the rescue itself: it is Verkstead
prodding an agent rather than anything the work has got to, and the line is in
the session's own Capture.
_Avoid_: Nudge (that is the viewer's signal), retry, reminder, ping, poke

**Stalled**:
A Conversation in a driven state — Grilling, Implementing, Wrapping or
Follow-up — with nothing registered as driving it and nothing on it saying it
is **Stopped**. Nothing is moving the work and nothing is saying so, which is
the one condition Verkstead has to notice on its own account. A condition an
active state can be in rather than a state of its own — the Conversation is
still Grilling or Implementing or Wrapping or Follow-up, and that is the half
of it that is wrong.

The condition rather than the record of it. A sweep looks every minute and
**stops** what it finds, by circumstance rather than by anybody's decision, so
what the human sees is a Conversation stopped with a Notice on it and a Resume
to press — and what a restart sees is one it may take up unasked.

Judged by whether a driver — a grilling session, the watcher a Pick armed on
one, a runner loop, a wrap-up's watchers, the task seeing a follow-up's session
out — is registered, rather than by how long nothing has happened. Wrapping
idles for days under live watchers and is perfectly healthy; so are the gaps
between an unattended run's Steps. Draft, Done and Closed are never stalled,
nothing being supposed to drive them.
_Avoid_: blocked on you (the badge a stall is precisely without), stopped (what
a stall becomes, not what it is), state, stuck, hung, idle

**Archived**:
A Closed Conversation the human has taken off the sidebar, there being nothing
left to read on it. A fact about the list and nothing else: the Timeline, the
branch and the Conversation's own page are exactly where they were, and opening
it by its URL shows all of it. Offered on a Closed Conversation alone — a
Conversation still being worked on belongs on the list it is being worked from,
so it is closed first and archived after. **Close and archive** is that order
in one press, for a Conversation the human is finished with and finished
looking at; it refuses what closing refuses and nothing more.

Reversible, which is what tells it from **Locked**: nothing is confirmed,
because nothing is lost. The two words are not each other's — one is a Question
Set settling for good, this is a Conversation leaving a list.

Two ways back, and they are different things. **Unarchive** takes it out for
good, and the Conversation is on the sidebar again as it was. **Show archived
conversations** is a way of looking rather than a change to anything: with it
on, what has been archived is drawn in its ordinary place; with it off, it is
not. That is the human's standing choice rather than a device's, so it is kept
beside the archivings and read back on every load.
_Avoid_: locked (the Question Set word), deleted, hidden, closed (the state
being archived, not the archiving), done, restore or unhide (the word is
unarchive)

**Unseen**:
A Conversation Verkstead has told the human about and they have not looked at
yet. The only thing in the record about the person rather than about the work,
and kept on the server for that reason: a mark held in a browser would be one
their phone had never heard of, and news read on the phone would leave the
laptop's sidebar still calling for attention.

One thing writes it: the wrap-up that carries the work to **Done**, in the same
breath as the push it sends about it. A milestone nobody was watching happen is
what a mark saying *look here* is for — and a **Steer** to Done is the human's
own act, so it pushes nothing and marks nothing. Opening the Conversation is
what takes the mark away, said by the browser in a call of its own rather than
happening on the way past a read — and **Closed** takes it away too, that being
the human saying the work is over wherever it had got to: nothing about a Closed
Conversation asks for them, the news it was carrying included.

Drawn as the same accent disc a Conversation waiting on the human carries: one
mark meaning *look here*, because two would be a list to decode instead of one
to glance down. Which of the two it is is in the row's read-aloud label, and
where both are true the waiting is what is said — a Conversation with something
to answer is asking for a reply, one with news is only asking to be read.
_Avoid_: unread (nothing here is a message), badge, alert, notification (that is
the push, this is what is left behind it), new

**Blocking Ask** / **Store-and-nudge Ask** / **Deferred Ask**:
The three ways an agent puts a Question Set to the human. A **Blocking Ask** idles
the session until the Response arrives, as every ask does in askance. A
**Deferred Ask** does not idle it: the Set waits in the Timeline and its
Answers are folded into a later session's prompt. Work blocks only on Questions
whose Answers affect work about to be done.

`verkstead ask --deferred` is the third one, and that difference is the session's
alone: it is the agent saying it will carry straight on, on every backend. Both
land on the Timeline, both leave the Conversation *blocked on you*
and both notify the human's devices; a deferred one says on the Timeline that it
is deferred, and its badge says no agent is waiting rather than that one has
disconnected. What is deferred is how it was asked rather than anything in the
Set, so it is kept beside the stored body rather than in it.

A **Store-and-nudge Ask** is the two halves the other way round, and is the
ordinary ask on a backend that cannot hold a shell command open for hours: the
Set is *stored* as a deferred one is, so `verkstead ask` returns at once and the
session ends its turn — and a session is *idling* on it all the same, waiting
for the line Verkstead types into its terminal when the Response lands, which it
answers by fetching the Answers with `verkstead answers`. Which of the two an
ordinary ask is is the backend's fact rather than the Set's: the CLI asks the
same way everywhere, and the server reads the agent type of the session that
asked. What the human sees is a deferred-shaped ask, because nothing is holding
a connection open on it; what differs is underneath, where everything that
decides a session's fate — the quiet grace, the Rescue, a wrap-up's proposals,
the locking of what a gone session left open — counts it as a question somebody
is standing behind.

The **nudge** in that name is one line typed into the session's terminal, down
the channel the Rescue types through and a watcher's keystrokes take — not the
viewer's **Nudge**, which is a signal to a browser and reaches no agent. It is
written as the human would write it and names the Set and the command that
fetches it, because a session that has ended its turn has neither in front of
it; the line and the Enter behind it are typed a moment apart, an interface
reading a burst as a paste. It is typed wherever the Response arrived from and
only where there is a session idling on that Set to type it at. Nothing goes on
the Timeline for it — it is Verkstead speaking to an agent rather than anything
the work has got to — and the session's own Capture holds it, the same account
the Rescue gives of itself.

The **folding** is the far end: when a session is started to build, every
answered stored ask of that Conversation nobody has been told about goes into
its prompt, oldest first, under the documents the prompt is built from — a
Deferred Ask, which had nobody to tell from the start, and a store-and-nudge one
whose session went before it fetched. Each is
folded once, and that it was folded is recorded rather than worked out from what
is answered. The one session never folded into is a relaunched grilling, which
is already primed with everything the Conversation has answered.
_Avoid_: sync/async ask, hard/soft question, urgent question

## Question Sets

**Question Set**:
A batch of Questions submitted together by one agent, with a Preface and a
title. The unit that lands on a Conversation's Timeline, gets answered, and is
locked. Reached through the Conversation it was asked from and nowhere else:
a second way in would be a second thing to keep true.
_Avoid_: request, batch, ticket

**Unreadable**:
What a stored Question Set is when the build looking at it cannot deserialize
the body it was written as — ordinary schema movement, a field having left. It
is drawn as a row saying so, on the Timeline it has always been on, with the
stored body reachable and nothing offered to answer or lock it by. The rule
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
The uncommitted changes (including untracked files) of the Worktrees a Question
Set was asked from, read by the server as it arrives and attached to it, so code
approval can happen in the web UI. The server knows which Conversation a Set was
asked from and can read those Worktrees itself, so nothing about it is taken on
trust from what was sent.
One block per repository a session may write in — the Conversation's own first,
then each read-write Companion Repo — each named by its Repo. A repository with
a clean Worktree contributes no block, and every one of them clean is a Set with
no Diff. The page draws the blocks under one *Diff* heading and labels every one
of them but the Conversation's own drawn alone — an unlabelled block means the
work's own repo, so naming it there would be naming it twice, and a companion's
block is named however alone it is. Which is the rule a commit card follows,
asked the same way round.
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
the gates Topic was retired the core is the whole of it. **One document,
tailored at print time**: how an ask is run and what comes back from it is the
backend's own, so those two sections come one per channel and everything about
writing a Set is written once. Which channel is the reader's own comes off the
agent type Verkstead sets in every sandbox; a Guide printed outside one is the
blocking Guide.
_Avoid_: help, manual, docs

**Topic**:
A task-scoped section of the Guide, split out so an agent pays its reading
cost only when the task is at hand — at which point it is required reading,
never optional. There are none at present: the gates Topic was the only one,
and it went when the last gate did.
_Avoid_: section, chapter

**Locked**:
What a Question Set becomes once it is settled — its Response delivered, or a
Set nobody will ever answer put away unanswered: by the human, from the page it
is on, or by Verkstead where the asking is over — a grilling relaunched over the
session that asked, or the Conversation itself closing. Permanent decision
history, and no place of its own to browse: a locked Set stays on the Timeline
it was asked from, saying what became of it, and takes no Response ever again.
Locking one by hand is the single irreversible act in the workbench, which is
why it is the single one confirmed in as many words. Nothing leaves a Timeline.
_Avoid_: archived (**Archived** is a Conversation coming off the sidebar, which
is reversible and about a list rather than about a Set), filed, history, log

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

Not the nudge in **Store-and-nudge Ask**, which is a line of English typed into
an agent's terminal. This one is a signal to a browser and never leaves the
viewer.
_Avoid_: tick, refresh signal, ping, change event, notification

**Update Notice**:
The banner the viewer shows when the server has learned that a newer release
exists than the one it is running, linking to the update instructions. Informs
only — nothing is installed on the human's behalf.
_Avoid_: upgrade prompt, new-version alert
