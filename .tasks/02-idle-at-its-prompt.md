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

## What was built instead, and why

**Codex has no at-the-prompt line.** Driven for real — codex 0.149.0, against a
stand-in model server, there being no account on the machine — the frame it
leaves when it is waiting for a human and the frame it draws mid-turn are the
same screen but for one line, and that line is the one it draws while it is
*working*:

```
25|• Hello from the stand-in.            waiting
28|› Ask Codex to do anything
30|  gpt-5 default · /tmp/codextrust/wt

25|◦ Working (39s • esc to interrupt)    mid-turn
28|› Ask Codex to do anything
30|  gpt-5 default · /tmp/codextrust/wt
```

Two measurements went with it, both the other way round from what ADR-0011
assumed: at its prompt codex sends not one byte once the frame settles, and
mid-turn it repaints every 33 ms without a gap.

So the constant is codex's **at-work** line — `esc to interrupt` — and a
signature now reads one of two ways, which way being a fact about the backend.
Put to the human as Question Set 1 (Q1), who took the recommendation: the
at-work line, with the ordinary three-second quiet asked for beside it, so that
a wording which drifts leaves a working session alone rather than reaping it.
ADR-0011's TUI-idle section records the whole of it. The at-the-prompt reading
and its tests are untouched, for the backends that will draw one.

The same round settled two things beside it: the trust pre-seed task 01
shipped is ignored when the Worktree path holds a dot, and is fixed here (Q2);
and the ADR is amended in this commit rather than later (Q3).

## What is still waiting on the human

**The account.** Everything above is the real codex binary drawing its real
frames, but no session has ever reached a prompt under a logged-in account
here. What that leaves unconfirmed is narrow — whether a subscription or
API-key login changes what the composer and the bar under it draw — and it is
the same wait task 05 carries.
