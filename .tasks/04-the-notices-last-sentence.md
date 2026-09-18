# 04. The Notice's last sentence

## What to build

A stop Notice draws two blocks of evidence: the worktree, and *What the last
session said*. The second reads off the session's Transcript, falling back to
its Capture where the backend kept no log, and where both are empty it says **It
said nothing at all.**

That sentence was true of the reporter's session and cost them an hour. A
desktop-app `claude.exe` is an Electron launcher: it starts, prints nothing and
exits immediately. Everything about that is in what the relay saw as the child
went — a non-zero exit, a lifetime measured in tenths of a second, and not a
byte printed — and none of it reached the Notice. An instant exit named as an
instant exit points straight at the binary; *it said nothing at all* points
nowhere.

So where the Transcript and the Capture are both empty, the Notice says instead
how the session ended: its exit code and how long it lived — *exited with code 1
after 0.4 s, having printed nothing*. The relay knows all of it as it reaps the
child; what it does not do today is carry it to where the Notice is composed.

**What already exists and what does not.** The relay reads the child's exit
status and already words the two ways of ending badly, and that wording becomes
the *reason* the Notice opens with. Missing are the session's lifetime, and the
account reaching the evidence block rather than only the reason — the stop is
written from the Conversation and the Event it was printing into, and neither
carries how the session ended.

Pick the route that puts the sentence there without threading a new argument
through every caller that writes a stop. Recording the ending against the
session's Event, where the evidence is already read from, is the shape that
fits; carrying it on the relay's report and parameterising the stop's fallback
text is the other. Either is fine — what matters is that the sentence appears
only in place of *It said nothing at all* and nowhere else.

**A session that said anything at all is unchanged.** Where there is a
Transcript, or a Capture with something in it, the evidence block is what it has
always been — the agent's own prose is better evidence than an exit code, and
this never displaces it. A session Verkstead ended on purpose is unchanged too:
that is not a session that went wrong.

## Acceptance criteria

- [ ] A stub session that exits immediately having printed nothing yields a stop
      Notice whose *What the last session said* block names its exit code and
      how long it lived, in place of *It said nothing at all*.
- [ ] A session that left a Transcript, or a Capture with anything in it, gets
      the evidence block it gets today, unchanged.
- [ ] A session Verkstead ended itself is unchanged, and the Notice's opening
      reason is unchanged in every case.
