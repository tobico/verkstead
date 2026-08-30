# 02. Grok Build judged idle by its screen

## What to build

Grok Build's idle signature, so a Grok session is judged by the frame it draws
rather than by a silence a full-screen TUI never keeps.

Stage 02 built the whole judgement and stage 03 corrected it: a backend is read
either by what it prints or by what it draws, a drawn reading is one constant
per backend, and that constant says one of two things — an at-the-prompt line
*standing* says the session has stopped, or an at-work line *gone* says it. A
byte-quiet long-stop measured in minutes sits behind both. What is missing is
Grok Build's one constant, and which of the two readings it is.

**This is second in the stage, not last.** Everything after it needs a session
that can be driven to a stop: Rescue's precondition is idle, every ender waits
on the same judgement, and no session carries a cap on its life. A Grok session
with no signature runs until the long-stop catches it, which makes every later
task a five-minute round.

**Both halves are read off the real thing.** Which reading Grok Build takes is
a fact about its screen rather than a choice, and codex came out the opposite
way from what the ADR assumed — so drive a real grok, look at the frame it
leaves when its turn is over beside the frame it draws mid-turn, and take the
constant from the difference. Measure the silence too: whether it is byte-quiet
at rest is what decides whether the ordinary three-second quiet can be asked
for beside the line. Guessing produces exactly the drift the long-stop exists
to catch.

Grok Build may well be the first backend that reads at-the-prompt. That path
and its tests already exist, written for a backend that would draw one, and
have never run against anything but a stub — so expect to find whatever a real
first use finds.

The constant goes where a backend's per-type answers already live, the same
bargain the usage-limit phrase makes: the wording is grok's and will move, and
moving it should cost one edit.

**This needs a `grok` on the machine**, and reaching a prompt needs an account
— an unauthenticated grok draws its sign-in screen. If either is still missing
when this is worked, take the signature as far as it goes without one and
record the confirmation against the real frame as waiting, rather than shipping
a constant nobody has seen stand.

## Acceptance criteria

- [ ] Grok Build's signature is one constant, kept where a backend's per-type
      answers already live, and the server reads it for a Grok session rather
      than falling back to what a test stood there.
- [ ] A real Grok session sitting at its prompt is judged idle within a round
      of the pace; one mid-turn is not, however long its redraws pause.
- [ ] With the signature deliberately wrong, the session reads as busy until
      the byte-quiet long-stop, and what the human then gets is the ordinary
      would-not-ask stop rather than a session that runs for ever.
