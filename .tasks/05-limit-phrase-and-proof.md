# 05. Codex's limit phrase, and the whole thing proved

## What to build

The last of Codex's own constants, and the stage's demonstration.

**The usage-limit phrase becomes the backend's.** Recognition today is one
sentence, claude's, matched off the Capture and the Transcript by a watcher
that knows which Profile a session runs under but not which agent it is. Codex
opens its own with "You've hit your usage limit" and decorates what follows by
plan, so the stable prefix is what gets matched — one phrase per backend, kept
in the one place, read off both records exactly as today. The rule that a line
must *say* it rather than mention it does not change: it is what keeps a
session grepping this repository from stopping its own run.

Nothing else about the stop changes. It is the ordinary stop — one Notice, one
*blocked on you*, one Resume — naming the Profile whose account ran out, and it
ends the session for the reason it already does.

**And then the stage is proved end to end**: a Conversation grilled, built and
wrapped under a Codex Profile. That is where the pieces meet — the launch line,
the signature that says it has stopped, the rollout on the Timeline, and
store-and-nudge asking on the channel Codex's type names. Watch two things in
particular, since neither has ever run against the real thing:

- **The nudge landing in codex's composer.** Codex merges keystrokes arriving
  in a burst into a paste, and a paste's return is a line break rather than a
  send — the exact failure the gap before the Enter was written for. If a nudge
  is swallowed, codex has a setting that turns burst detection off and the
  launch line is where it goes. Leave it alone if the nudge lands.
- **The Guide a Codex session reads.** It should be the store-and-nudge one,
  and the session should end its turn on an ask and come back for its Answers
  when the line arrives.

Bring CONTEXT.md's wording up with what this lands — the usage-limit phrase is
now one per backend rather than one, and a Transcript's log is found rather
than named on a backend that names no session.

**The proof needs a logged-in Codex account.** There was none on the machine
when this stage was planned. Everything that can be built and tested without
one still is; the end-to-end run waits on the account rather than being
declared done without it.

## Acceptance criteria

- [ ] A Codex session printing its limit line stops the run with a Notice
      naming the Profile, read off the Capture and off the Transcript both, and
      Claude's phrase and stop are unchanged.
- [ ] A grilling under a Codex Profile asks by store-and-nudge, ends its turn,
      is nudged when the Response lands, fetches its Answers and carries on
      from them.
- [ ] A Conversation is grilled, built and wrapped end to end under a Codex
      Profile, and CONTEXT.md says what this stage changed.
