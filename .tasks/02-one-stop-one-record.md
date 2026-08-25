# 02. One stop, one record

## What to build

The Halt and the Pause become one thing: a Conversation is **Stopped**, however
it got that way. The state moves onto the Conversation row itself rather than
into a table beside it — which is what makes *at most one open stop per
Conversation* a fact about the record rather than a rule something has to
enforce.

What the Conversation carries:

- **when it stopped**, and nothing at all where it has not;
- **whether anybody decided it** — deliberate where Verkstead pulled the brake
  or the human pressed Stop, circumstance where a restart or a crash took the
  driver away. Unchanged in substance: a restarting server takes up only the
  stops nobody chose and leaves deliberate ones waiting;
- **the Notice** that explains it — what stopped, why, and the evidence;
- **the reset words**, where the stop carries any: the line the session printed
  about its account being out of window, kept as text to show rather than as a
  time to act on.

The asked-for Stop that has not landed yet moves the same way, for the same
reason: one Conversation asks to stop once.

Every reader and every writer then asks one question about one thing — the
guard in front of every launch, the stalled sweep, the restart's own resume,
the sidebar's *waiting* mark, the Conversation view the workbench draws, and
the ten modules that write a halt today, all of which already go through one
function.

**The window's stop joins them here.** The usage-limit watcher stops writing a
Pause Event and writes an ordinary stop instead: a Notice naming the Profile
that ran out and carrying the line the session printed, with the reset words on
the stop. What it does *not* change yet is the self-resuming wait — the sweep
that ends one when its reset passes still runs, and clearing it is task 03's.

**What is already stored reads back as a stop.** When the schema is applied,
every open halt and every open Pause is copied onto its Conversation. The halt
and pause tables are left exactly where they are and no row in either is
rewritten — they are the record of what happened, and an old Pause Event stays
on its Timeline.

Naming: the state is **Stopped**. *Stop* is already the two presses and the
module that handles them, so the state needs a name of its own rather than
theirs.

## Acceptance criteria

- [ ] A Conversation stopped by an exhausted window and one stopped by a press
      read back as the same kind of thing, with one Notice and one badge
      behind each.
- [ ] A Conversation carries at most one stop however it arrived, and nothing
      is launched past one.
- [ ] A database written before this opens with its open halts and open Pauses
      already reading as stops, and with neither of those tables changed.
- [ ] A restarting server takes up only the stops nobody decided.
