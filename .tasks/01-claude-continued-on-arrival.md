# 01. Claude continued on arrival

## What to build

The whole shape of the stage, cut down to one harness so that the first slice
is a whole one. A Conversation that lands on this device, whose last session
ran on Claude, is started with **the harness's own resume** against the log the
memory sync carried over — rather than the fresh, re-primed session stage 09's
arrival starts. Every other harness, and every case where the resume cannot be
made, falls through to Verkstead's Resume exactly as it stands: this task adds
a way in beside the standing one and changes nothing about it.

**The launch line grows a resume shape.** The one value that says what a
backend's command line looks like — how it is told its model, how it is given
the Brief, whether it takes the session id Verkstead named, what goes on the
end — grows the other line the same backend has: the one that continues a
conversation instead of opening one. Claude's is the resume flag and the id,
with the prompt staying where it already is, and it is **mutually exclusive
with the session-id flag**: a session cannot be both named and resumed, so a
resumed launch names none and the resumed conversation keeps the id it already
had. That matters beyond the line — the record of what Verkstead called each
session is what the Transcript is looked up by, so the row written for the
resumed session's Capture is the *old* id rather than a fresh one, or the log
of the session that is actually running would not be found.

**What it is resumed by is the last session of this Conversation.** The record
already holds every name Verkstead has given this Conversation's sessions, in
order, and the newest of them is the conversation the agent was having when the
work moved. It is only usable where the Pairing the resume is about to launch
under is the same harness that session ran on — a Conversation whose Profile
was changed under it has a log no other backend can read.

**And the log has to actually be there.** Claude's store is carried across by
the memory sync as a labelled part that each machine names its own path for, so
it should land under this device's own encoding of this device's Worktree path
with nothing further to do. Prove that rather than assume it: the sync names
the Worktree's part off the Data Directory and the stem the Worktree directory
was named with, and a stem that came out differently on this device — the plain
one having been taken here — is a part that was asked for under a name the
sending device never used. Where the log is not found, this is Verkstead's
Resume and nothing is lost.

**The prompt is the note alone**, not the note over the ordinary re-prime. The
agent has its own context and is mid-turn in its own conversation; being told
again what it already knows is worse than being told nothing. The note names
the device the work now runs on and the Worktree's new path, and that is the
whole of it until task 02 adds the Sets.

**The Transcript starts where the carried log ends.** Claude appends to the
same file, so a Tail that began at the top would write the whole of what the
session on A said onto the new session's Event — the same conversation drawn
twice across the two machines. Each Event holds its own session's words, so the
following of a resumed log opens at the end of what crossed.

## Acceptance criteria

- [ ] A grilling transferred mid-interview comes up on the receiving device as
      a resume of the same Claude session and carries on from the question it
      was on, rather than re-opening the interview.
- [ ] The session is primed with a note naming the device it now runs on and
      the Worktree's new path, and with nothing else.
- [ ] The resumed session's Transcript holds what that session said and not
      what the session on the sending device had already written into the same
      log.
- [ ] No log, no session on the record, or a Pairing on another harness leaves
      Verkstead's Resume doing exactly what it does today.
