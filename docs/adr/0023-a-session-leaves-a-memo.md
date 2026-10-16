# A session leaves a memo for the sessions after it

Every session Verkstead launches on a Conversation starts with none of the
context of the ones before it. The next-task skill says so outright, and it is
the point: one task per session, read off the backlog, with nothing of the
previous session's head in it. What crosses between them is what was committed
— the code, the ticked box, the commit message — and the documents the prompt is
built from: the Brief, the handoff where there is one, the human's answers to
the deferred Sets.

What does not cross is what a session *learned*. A task session finds that the
sessions suite flakes under parallelism, or that a helper the task file names
has moved, or settles a shape nobody wrote down because the task did not ask —
and the session after it finds the same thing again, or does not. The human
reported where that went: implementation sessions were discovering a small
detail that mattered to the next task, and telling it to *them*, over a
Question Set, for them to carry forward by hand.

Three things almost carry an agent's own words forward, and each stops short.
The **Handoff** is written once, by the grilling, and only on an inline pick.
A **Deferred Ask**'s answers are the human's words rather than the agent's, and
are folded into one prompt and then marked folded. The harness's own memory is
per account and per Repo, synced across devices, and never per Conversation.

## The decision

**A session leaves a memo, and every later session of the Conversation reads
it.** `verkstead memo` takes markdown on stdin and puts it on the Timeline;
every session started on the Conversation afterwards is told it, under its own
heading in the prompt, until something retires it.

**What a memo is for is working knowledge no artifact holds.** A flaky test, a
build trap, a decision taken mid-task and why, a thing the next task will walk
into: short, specific, written when learned. Not a running account of the work
— the commits are that — and not anything at all. The skills say what belongs
in one.

**Only a building session may leave one.** The task, finish, inline,
steered-instruction and stage-planning sessions: the ones that will have a
session after them with none of their context. A grilling has the handoff and a
wrap-up has the pull request, and both already report the way their readers
read. Any other session's memo is refused by name, saying which may — the
server knows what every running session was sent for, because the Done signal
is checked against it.

**Every later session reads them.** Grilling relaunches and steered sessions
included, which is where this differs from a deferred Set's answers: those are
folded once, into the one session they were meant for, and a relaunched
grilling deliberately gets none. A memo is not spent by being read. A Stage of a
roadmap is a Conversation of its own and reads its own memos; its brief carries
what the roadmap settled, and a memo about the planning branch is rarely about
the stage. A Conversation carried to another device keeps its memos with the
rest of its record, and the note a carried session is primed with on arrival
says nothing of them: that session was launched after they were left and has
already read them.

**A list, not a document.** One Event per memo, each stamped with the session
that left it — *the session that worked task 03* — and numbered by its Event
id, the way a Question Set is by its Set id. A session writes one with one
command and never reads the list back: the prompt is where it reads them. A
single document each session rewrote would cost every session a read and a
rewrite of the whole, and would lose who said what. The list is shown in full
in every later prompt, oldest first, and outlives the round: a steer into
Grilling opens a new Brief and leaves the memos standing, because what a
session learned about the branch is true of the branch rather than of the
round.

**Taken back by a later session or by the human.** A wrong memo would poison
every session after it, so `verkstead memo --retire <id>` takes one back, with
an optional reason, and so does a press on the card. A retired memo leaves the
prompt and stays on the Timeline marked retired, with who and why — the Timeline
is a record, and a Set closed unanswered stays on it the same way. The human's
press is undone by Restore, so it asks nothing first.

**A Timeline Event, because nothing happens off the record.** A Handoff-shaped
card, the whole memo in the details pane, and no place on a Share: a memo is one
agent talking to the next, like the handoff and the session's own output, which
a Share leaves out. The human cannot leave one from the Timeline yet — a Steer
and a deferred Set's comment box already carry their words to a session, and
this is the agents' — and nothing here stands in the way of adding that.

**And the Timeline read becomes tolerant of a kind it does not know.** This is
the first Event kind added since cluster mode, and a record carried to a device
on an older build lands there with a kind that build's read refuses outright,
taking the whole Conversation's Timeline with it. The read now draws an unknown
kind as a plain Notice saying what it is and goes on. The tolerance protects a
device only once it runs a build that has it: a device still on the build before
this one shows a broken Timeline on the first memo until it is updated, and the
fix pays off from the kind after this one. Taken anyway, because the
alternative — a side table drawn on the Timeline from outside the record — would
be the one thing on a Conversation that happens off it.

## What was considered

**A document each session rewrites.** One page the newest session decides the
contents of, kept current by rewriting. Rejected for the cost above, and
because it makes the newest session the editor of every earlier session's
words.

**A list the wrap-up summarises.** The list, plus a fold into one page at the
wrap-up. Rejected as a second mechanism for a problem the list and retiring
already answer; nothing stops a wrap-up retiring what no longer holds.

**Shown once and then marked delivered**, the way a deferred Set's answers are.
Rejected: a memo is not an answer to one session's question but a fact about
the branch, and the session after the next needs it as much.

**A per-Conversation ordinal** — Memo 1, Memo 2 — stable across a Transfer.
Rejected as a second numbering nothing else on the record uses; a Transfer
renumbers the Event id with everything else, and every later prompt names the
new one.

**Leaving the refusal to the skills.** Document the command in the building
skills alone and refuse nobody. Rejected: the server already knows what each
session was sent for, and a rule the server can hold is held.

**Calling it a note.** The human's own word for it, and the obvious one.
*Note* is already the text a carried Conversation is primed with after a
Transfer, and *notes* is on the Handoff's avoid list. *Memo* — a thing written
to be remembered by a later reader — is what it is, and nothing in the glossary
uses it.

## Amended: a memo has a title, and the card shows it

*Amended 2026-10-16.* The card above was the handoff's: headed by the Event id,
stamped with the session, three lines of the body, and Retire at the end of the
head. Read down a record, a column of those was a column of numbers over the
openings of paragraphs, and the number told the human nothing — it is the
session's handle for retiring, never theirs.

**A memo is named by its session.** `verkstead memo <title>` takes the title as
its one argument, the way `verkstead preview <title>` already does, with the
markdown still on stdin. A title is trimmed and refused empty or with a newline
in it, and has no length cap: the card cuts it to one line. Memos left before
this have none, and the card shows the body's first line in its place rather
than a migration inventing one.

**The card is the kind and the title.** Headed *Memo*, the title as its
subtitle, and nothing else: no stamp, no body, no button. A retired one strikes
the subtitle through and says no more; who took it back and why are in the pane.
The number is drawn nowhere on screen.

**The pane says the rest.** Headed by the title, the whole memo under it, and
under that an understated line — who left it and when, the session's name a
link that opens that session's own output, and *Retired by … reason* beneath it
where retired. Retire, or Restore, stands beside that line rather than in the
head: it is the one act on the memo, and it belongs with the words that say
where the memo stands.

**The prompt names it by the title too.** `**<title>**, left by the session that
worked task 03.`, and the id only in the line that says how to retire it, which
is the one place a session needs it.

**And a memo after a Set no longer walks the selection past the Set.** The
Timeline's selection followed the end of the record, so a memo landing after a
Question Set opened the memo and left the Set unanswered behind it. The landing
is now the pending steer, else the oldest Set still waiting — blocking,
disconnected or deferred alike — else the end of the record; a Set locked
unanswered is passed over, and a share lands on its end as before. Answering
from the pane puts the Timeline back into advancing, so a phone that tapped the
Set's card to answer it is carried on to the next thing.

Considered and rejected: **the title as the markdown's first line**, which
makes a heading the body cannot have and a memo with no heading untitled;
**a `--title` flag**, which makes an untitled memo the default; **backfilling
old memos** from their first line, which writes titles nobody chose; and
**Retire in the pane head**, where it reads as chrome rather than as an act on
what is written.
