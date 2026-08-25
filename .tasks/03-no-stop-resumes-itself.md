# 03. No stop resumes itself

## What to build

Every stop ends by a press. The recognition of an exhausted usage window stays
exactly as it is — one phrase, read off what the session printed and off the
log its backend wrote — but the five-hour wait behind it goes:

- the sweep that ended a wait when its reset time passed, and the pace knob it
  ran on;
- the record of *what* ended a wait, there being one way left;
- the reading of a reset time as a moment: the clock times, the instants
  written whole, the question to the machine about the offset it keeps local
  time at, and the tests that covered all of it. What is stored is the line's
  own words, which is what the card shows.

**And the stop ends the session it stopped**, which is new. Verkstead has never
touched that session: the agent holds it at the limit and carries on by itself
when the window comes back, and what kept the two in step was Verkstead's own
wait firing at the same reset and relaunching. With the wait gone, an agent
that woke at the reset would work on inside a Conversation that reads as
stopped, and the press that came the next morning would launch over whatever it
had done.

Mind how that ending is done. The watcher that recognises the limit runs
**inside the relay task of the very session it is ending**, and the ordinary
way of ending a session waits for that relay to finish — so an ordinary call
there waits on itself for ever. The stop is written first and the ending
follows it, the same order a Force stop uses and for the same reason: a session
Verkstead ended advances nothing, so the driver seeing it out goes straight to
its next launch, and the stop has to be there when it looks.

The notification rules do not move. A stop for an exhausted window is one
Verkstead decided on, so it is pushed, naming the account and when it comes
back; the human's own press and a stop nobody chose still send nothing.

## Acceptance criteria

- [ ] Exhausting a usage window leaves the Conversation stopped with the
      Profile and the reset words on it, and no session running behind it.
- [ ] Nothing resumes on a clock. The reset passing changes nothing, and the
      one Resume clears the stop whatever wrote it.
- [ ] Nothing anywhere reads a reset time as a moment, and the server keeps no
      timer for a stop.
