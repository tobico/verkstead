# 05. Screen-signature idle, and the byte-quiet long-stop

## What to build

One judgement about whether a session is idle, made per agent type, and read by
everything that has ever asked. Today the question is answered by a clock over
the bytes a session prints: three seconds of silence is idle. That was
calibrated to an agent drawing inline and does not carry to a backend that
repaints a full screen for ever — such a session is never silent, so nothing
would ever end it, rescue it or mark it idle in the sidebar.

**For a TUI backend, idle is the drawn screen.** Verkstead already holds the
Screen — the grid the session's bytes leave on a virtual terminal — so what
says a session is at its prompt is that backend's own at-the-prompt signature
appearing on it. One constant per backend, kept where the usage-limit phrase is
kept and accepted to drift the same way: the wording is the backend's, it will
move, and it costs one edit when it does. No signature ships in this stage —
Codex's is stage 03's, and the suite's stub carries its own.

**The three-second mark does not count as idle there.** A TUI that falls silent
for a moment mid-turn would otherwise read as idle and be reaped out from under
its own work.

**But a long byte-quiet does, and has to.** A signature that has drifted is a
session nothing catches: Rescue's precondition is quiet, every ender gates on
the same clock, and no session carries a cap on its life — so it would run for
ever, holding its Worktree, with the backlog stopped and nothing in front of the
human. So byte-quiet stays as a long-stop, **five minutes**, sitting in `Pace`
beside the sixty seconds the enders and Rescue already wait on and settable for
the suite the way the rest of `Pace` is. A session past it is idle whatever its
screen says, and what the human then gets is the ordinary would-not-ask stop.

**Claude stays on the three seconds**, unchanged, measured on what it was
calibrated for.

**One clock, and both readings of it.** Every reader of byte-quiet moves to the
judgement, and there are two kinds of reader, not one:

- *How long has it been idle* — the idle mark in the sidebar and on the
  Conversation, the enders that wait out the grace, and Rescue's own wait.
- *When was it last busy* — Rescue's proof that a stir landed, which is a word
  arriving after the answer or the line was handed over. A byte is free on a
  backend that repaints, so that proof has to be the same judgement read as a
  moment: the session was last seen busy after it was stirred.

No caller keeps a private byte rule afterwards.

## Acceptance criteria

- [ ] A stub that repaints continuously is never byte-idle, is judged idle when
      its at-the-prompt signature is on the Screen, and both its ender and
      Rescue act on that judgement; a three-second byte silence mid-turn does
      not end it.
- [ ] With the signature taken away, the same stub reads busy until the
      five-minute long-stop, after which the ordinary would-not-ask rules prod
      it twice and stop the Conversation with its Notice.
- [ ] A Claude session is judged idle after three seconds of byte-quiet exactly
      as today, in the sidebar, in the enders and in Rescue.
