# One stop, and Steer

Amends [ADR-0007](0007-server-held-terminal.md): the Hold is retired.

The Conversation control model grew three overlapping ways of being stopped —
the **Halt**, the **Pause**, the **Hold** — plus three side doors around the
pipeline: **Manual Task**, **Reopen** and **Abort**. Each earned its place one
feature at a time, and together they cost more than they buy for a single-user
product: three stopped-shaped things to read on a card, and still no way to
work around an exception the pipeline did not foresee. Four real ones drove
this decision: a completed pull request the implementation missed, duplicated
work in a parallel branch that needed merging in, a manual review after the
automated one was interrupted, and follow-up work that belonged on the same
branch. None of them was expressible as anything but hand-editing state.

Two decisions replace the lot.

## One stop

A Conversation is **driven** or it is **stopped** — one concept where Halt and
Pause were two. A stop keeps its Notice (what stopped, why, and the evidence)
and remembers whether anybody decided it: a restarting server still takes up
only the stops nobody chose, and the notification rules are unchanged. What is
dropped is the self-ending wait: **no stop resumes itself.** A stop for an
exhausted usage window still names the Profile and shows the reset time the
session printed, but it waits for a press like every other — the reset time is
information, not a timer. *Blocked on you* badges any stopped Conversation.

And **a usage-window stop ends the session it stopped**, which is new. Verkstead
has never touched that session: the agent holds it at the limit and carries on
by itself when the window comes back, and what kept the two in step was
Verkstead's own wait firing at the same reset and relaunching. Take the wait
away and nothing does — the agent would wake and work on inside a Conversation
that reads as stopped, and the press that came the next morning would launch
over whatever it had done. So *no stop resumes itself* is made true of the agent
too, at the cost of the window the session would otherwise have worked through
unwatched.

The Hold goes too, and nothing replaces it. Typing into a Screen commits
Verkstead to nothing: no register, no hand-back, no badge, and no clock a
keystroke puts back. Somebody who wants the keyboard **stops the Conversation
first**, and the one stop is what holds the run off while they work — the same
act Steer makes for itself at click. A keystroke-resets-the-quiet-clock rule
was considered here and dropped: what a keystroke would buy is one grace
period, a number measured for an agent between prints, so it would read as
protection while giving somebody who pauses to think almost none. Two ways to
be protected was the thing this decision set out to stop having.

## Steer

**Steer** is the one hand-on-the-wheel control, and it subsumes the side
doors. A row in the Conversation's menu beside Stop: clicking it stops the
drive where it is — nothing new launches, and a running session is seen out
unless the modal's **Interrupt current task** ends it where it stands — and
opens a modal. Cancelling leaves the Conversation stopped, with Resume on
offer; the stop at click is deliberate, freezing the world while the human
composes.

The modal carries:

- **A target state** — Grilling, Implementing, Wrapping or Done — reachable
  from any state at all, recreating what is missing: a Worktree from the
  branch, the branch itself for a Draft.
- **Into Grilling**: an optional new brief, landing as a new round's Brief
  Event frozen at once, and a choice of whether the digest of everything
  already answered primes the session alongside it.
- **Into Implementing**: continue the existing backlog or roadmap where one
  stands, otherwise a hand-written instruction — required where nothing else
  says what to run. The instruction session *drives the pipeline*, unlike the
  old Manual Task: when it finishes cleanly, the pipeline carries on from
  whatever the branch then holds.
- **The Pairing** for the role being steered into, prefilled from the
  Conversation's own and **recorded as the Conversation's** — steering
  re-settles what runs the work, not a one-off.

Submitting clears any stop, moves the state, lands a Steer Event on the
Timeline carrying the payload, and resumes in the same press; a steer to Done
is the move alone, there being nothing to drive.

With Steer standing, **Manual Task and Reopen are retired** — an instruction
into Implementing is the first, and steering a closed Conversation back into a
state is the second — and **Abort is renamed Close**, which is what it always
was: the Worktree deleted, the branch kept, pressable from any state. A
Conversation is closed when its Worktree is deleted and back when a steer
makes it a new one.

## Considered Options

- **A bare state override** — the original brief. Moves the label without
  moving the work: the overridden Conversation still needs priming, a session,
  and its stop cleared, so every use would be an override plus three other
  presses. Rejected for the move-and-resume shape with payloads.
- **Patching each concept in place** — keep Pause's self-resume, keep Manual
  Task, add an override beside them. Every future exception grows another
  control; the card keeps three stopped-shaped things. Rejected wholesale by
  the human in the grilling.
- **Auto-resuming usage-window stops** (the status quo) — dropped: one resume
  rule for every stop is worth more than the convenience, and the reset time
  still reads as information.

## Amended: the form is a Timeline item

*(2026-09-23. This replaces the modal in *Steer* above, and nothing else in
it.)*

**The Steer form is an item on the Timeline rather than a modal over the
workbench.** A steer is often a lot of text, and a modal that blocks the
workbench while it is written is the wrong place to write it: the human wants
to read the Timeline, look at another Conversation, and finish the form when
they are ready. So the press that opened the modal writes a **pending steer**
beside the Conversation, the Timeline draws it as its last item, the form is
that item's details pane, and every field is saved to the pending steer as it
is typed — the draft Brief's shape, so it survives a reload and is found from
any device. Submitting freezes the whole form into the Steer Event, which now
records the digest and interrupt ticks, the pairing picked and the companion
rows asked for beside the target and body it always carried, and the details
pane draws the record as the form, read-only. The item stays as the record of
what was chosen.

**What stays.** The stop at the press, for the reason above: it is the one
stop applied to steering, and the world the form is drawn against is the
world the submit lands in. Cancel leaves the Conversation stopped with Resume
on offer, and posts nothing but the pending steer's deletion: a steer that
decided nothing is no Event, and the stop Notice already says the human
pressed. Submit lands the Steer and Moved lines as it did.

**What is new.** Resume is the opposite decision and discards the pending
steer before it starts; Close discards it as it shuts every open Set; Stop and
Force stop, being about the run, leave it. A second press on Steer selects the
pending steer rather than making a second. A pending steer counts as waiting
on the human — the sidebar disc and *Waiting on you* — and raises no
notification, the press being their own. The interrupt tick follows the live
Conversation rather than what the press found. A pending steer never boards a
Share.

**Kept beside the Conversation rather than as a Timeline row.** The Timeline
is ordered by row id, so a row written at the press could not be moved to the
end at submit without changing its id and breaking the address selecting it.
One pending steer per Conversation, drawn last by the page and opened at an
address of its own beside the share pane's and the terminal's; the record is
written at submit, so the Steer and Moved lines stay adjacent however long a
seen-out session went on landing commits under the press.

Considered and dropped: a device-held item drawn by the page, which no other
device would see and a reload would lose; stopping at submit instead of at the
press, which reverses the argument above and draws the form against a moving
world; and a Timeline row re-stamped at submit, for the ordering reason given.
