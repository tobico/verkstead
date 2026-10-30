# 01. The report

## What to build

A stage of a roadmap whose live copy is on a device other than its **home**
tells home every lifecycle move — into Wrapping, into Done, Closed, a stop and
the stop lifting — and home writes it onto the stage's tombstone, so that
everything at home that reads a roadmap reads the truth.

**Home is the stage's birth device** (decided in Set 1534): every stage is made
on the device driving its roadmap, so the Birth Key's device names home and
never changes — including for a roadmap adopted with *Continue a roadmap*, which
has no roadmap Conversation of its own. A Conversation is a *copy away from
home* when its birth device is not this device and it wears no transferred mark;
only a copy that is a stage (it has a roadmap and a stage label recorded) reports.

**What home reads, and so what the report has to keep true.** The standings are
not the state column alone: `stage_standings` also reads whether the
Conversation ever *moved to Done*, whether it ever *moved to Wrapping* (both
Timeline Events), and whether it is stopped. So the report carries the whole
standing — the state, those two facts, and stopped — rather than one move, and
home writes it so that the tombstone's row reads the same standing as the live
copy. Sending the whole standing each time makes a resend harmless. The chain
(the opened pull request) is task 03's and is not in this report yet.

**The link.** A new member-only route on the peer listener, beside the news
route that is its nearest precedent (a member saying something about its own
work), addressed by the Birth Key so home finds its tombstone. Home refuses a
report about a Conversation it holds no tombstone for, or whose tombstone names
another device as holding the live record — only the device the work is on may
report. A report that cannot be delivered because home is unreachable is kept
and sent when home next answers (on reconnect or a slow retry), never dropped.

**What home does with it.** A reported move that settles the stage runs home's
carry-on for the tombstone exactly as a local settle would, so the stage that
stood on it starts at home. The *what started* sentence is not said on the
tombstone (nobody can open it): home **answers the report with it, and the far
copy says it on its own Timeline** (decided in Set 1534). Read `docs/copy.md`
before writing it.

**The glossary.** `CONTEXT.md`'s **Birth Key** entry says nothing writes to a
tombstone; rewrite that sentence to say a stage's tombstone follows its live
copy's standing.

## Acceptance criteria

- [ ] A stage closed on the far device reads abandoned in home's standings.
- [ ] A stage that settles on the far device starts, at home, the stage that stood on it, and the far copy's Timeline says what home started.
- [ ] A move made while home is asleep reaches home's tombstone once home answers.
- [ ] A report from a device that does not hold the live record is refused.
