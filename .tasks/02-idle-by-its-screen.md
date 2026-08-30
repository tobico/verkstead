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

## What was read off the real thing

Grok Build comes out where codex came out: **an at-work reading**, not the
at-the-prompt one this task expected it might be. Measured against grok 1.0.13,
driven on a hundred-column terminal through the same virtual terminal the
Screen is — a throwaway probe on `avt`, so what was read is what
`Screen::showing` reads.

- **Its waiting frame is its working frame.** The composer, its `❯`, the
  `grok-4.6 · always-approve` label on its border and the `Shift+Tab:mode` and
  `Ctrl+x:shortcuts` hints beside it stand in both. What is there only while a
  turn runs is the live status line — `⠧ Responding… 5.7s … [stop]` — and the
  `Esc:cancel` hint on the row under the composer. Across a turn sampled once a
  second — a tool call and then a streamed reply, 27 working frames and 16
  resting ones — both were in every working frame and in none of the resting
  ones, and in neither of the two frames before grok had drawn anything.
- **The constant is the hint**, `Esc:cancel`: the hints are the row grok draws
  at the foot of every frame, where the status line is drawn only mid-turn, and
  a keybinding label is a harder thing to hit by accident in what a session
  printed than a bracketed word is.
- **It is byte-silent at its prompt**: not one byte in ninety seconds of
  sitting there. Mid-turn the widest gap between reads was 208 ms once it had
  drawn its first frame — 2.0 s once, between its first escape sequence and
  that first frame — so the three-second quiet asked for beside the hint is
  never met while it works.

## What is still waiting

There is no xAI account on this machine and no `grok` on the system profile, so
the frames above were read off a grok run **outside Verkstead**: installed under
a `GROK_HOME` of its own from `@xai-official/grok`, and pointed by
`GROK_XAI_API_BASE_URL` and `XAI_API_KEY` at a stand-in xAI Responses API that
streams replies slowly and calls `run_terminal_command`, so that a turn could be
held open and looked at. Everything on the screen is grok's own; only the model
behind it was not.

What that leaves outstanding is one criterion's real half:

- a Grok session **launched by Verkstead** under a real account, judged idle at
  its prompt within a round of the pace by the running server rather than by the
  suite's stubs. The signature itself is settled and the reading is proved, so
  what is left is the account.
