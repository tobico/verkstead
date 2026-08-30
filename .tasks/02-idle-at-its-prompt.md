# 02. Codex judged idle at its prompt

## What to build

Codex's at-the-prompt signature, so a Codex session is judged idle by the frame
it draws rather than by a silence a full-screen TUI never keeps.

Stage 02 built the whole judgement: a session's backend decides whether it is
read by what it prints or by what it draws, the Screen is already held and
already answers whether a signature stands anywhere on it, and a byte-quiet
long-stop measured in minutes sits behind a signature that has drifted. What is
missing is the one constant. Where a real server has Codex's answer, the code
currently reaches for the signature a test stood there.

This is second in the stage rather than last because everything after it needs
a session that can be driven to a stop: Rescue's precondition is idle, every
ender waits on the same judgement, and no session carries a cap on its life. A
Codex session with no signature runs until the long-stop catches it.

**The signature is read off the real thing.** It is one line of the drawn
frame — the composer as codex leaves it when it is waiting for a human — and it
has to be taken from a running codex rather than guessed at, because guessing
produces exactly the drift the long-stop exists to catch. It is kept in the one
place a backend's per-type answers are kept, the same bargain the usage-limit
phrase makes: the wording is codex's and will move, and moving it should cost
one edit.

**This needs a logged-in Codex account** — an unauthenticated codex draws its
sign-in screen and never reaches a prompt. If there is still none on the
machine when this task is worked, take the signature as far as it can go
without one and record the confirmation against the real prompt as waiting,
rather than shipping a constant nobody has seen stand.

## Acceptance criteria

- [ ] Codex's at-the-prompt signature is one constant, kept where a backend's
      per-type answers already live, and the server reads it for a Codex
      session rather than falling back to what a test stood there.
- [ ] A real Codex session sitting at its prompt is judged idle within a round
      of the pace; one mid-turn is not, however long its redraws pause.
- [ ] With the signature deliberately wrong, the session reads as busy until
      the byte-quiet long-stop, and what the human then gets is the ordinary
      would-not-ask stop rather than a session that runs for ever.
