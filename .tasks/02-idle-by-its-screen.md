# 02. OpenCode judged idle by its screen

## What to build

The one signature constant that says whether an OpenCode session has stopped,
read off the real TUI rather than guessed at, and the arm that hands it to the
idle judgement.

Without it an OpenCode session is judged the way Claude is — three seconds with
nothing printed — and that reading was calibrated on an interface that draws
inline. opencode draws a full screen at sixty frames a second, so a session that
paused mid-turn would read as idle and be rescued out from under its own work,
and one whose renderer keeps repainting at rest would never read idle at all.

**Which of the two readings it is, is a fact about opencode rather than a
choice.** A signature is either an at-the-prompt line, whose *standing* says the
session has stopped, or an at-work line, whose *going* says so once the ordinary
three seconds of quiet says so too. Codex and Grok Build both came out the
second way — each leaves the frame it works in and the frame it rests in
differing only by a status line — and the at-the-prompt reading has stood
unused since it was written. Measure opencode rather than assuming it joins
them: drive a real session, sample its frame once a second across a turn and
again at rest, and find what is in every working frame and no resting one, or
the reverse. Prefer a fragment that does not move — a keybinding label rather
than a spinner glyph or a seconds count.

**Measure the silence beside it.** The at-work reading asks for three seconds of
quiet as well, because the line is equally missing from a session that has drawn
nothing yet; the at-the-prompt reading does not. Whichever it turns out to be,
record what the terminal actually did — how long the widest gap between reads
was mid-turn, and whether it went byte-silent at rest and for how long. That is
what says the reading is safe, and it is what the two constants before this one
each carry in their own comment.

**Settle the alternate screen while the binary is in front of you.** codex and
grok each take a flag that keeps them drawing inline, and Verkstead passes it for
exactly one reason: the Capture is the record of what a session did, and an
alternate screen is a record thrown away as the program leaves it. opencode has
no such flag, and its renderer is built without the option being passed either
way. Find out what it does. If it takes the alternate screen, the Screen still
reads — the screen model already tracks one — but say plainly what the Capture
then holds, and whether the minimal interface opencode offers instead is worth
reaching for. This is a finding to write down, not a thing to work around
silently.

**The long-stop is already there and does not move.** A signature that has
drifted reads as a session that never stops, and what catches one is the
byte-quiet measured in minutes behind every screen-judged backend. Nothing here
adds to it; the proof is that it still ends a session whose signature never says
stopped.

## Acceptance criteria

- [ ] An OpenCode session running under a real Profile reads *at work* while its
      turn runs and *stopped* once it is over, judged off the frame, with the
      constant named for the release it was read on.
- [ ] A session whose signature never says stopped is still ended by the
      byte-quiet long-stop, and lands in front of the human as the ordinary
      would-not-ask stop.
- [ ] What opencode does with the alternate screen is settled and written down,
      along with what the Capture holds as a result; Claude's three-second
      reading and the two signatures already shipped are unchanged.
