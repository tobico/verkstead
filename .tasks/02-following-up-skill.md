# 02. Ship the following-up skill and its prompt

## What to build

A **following-up** skill beside the shipped ones (responding, reviewing,
instruction), and the prompt builder that dispatches a session into it:
"Read <skill> and …" over the Conversation's documents — the Brief and the
handoff where one exists — with the human's follow-up brief last, under a
heading of its own, the way an instruction session's prompt carries the
instruction. Deferred answers fold in exactly as they do for every other
dispatched prompt.

What the skill says:

- **The brief is written to this session.** Act on what it plainly asks —
  answer its questions, do the work it requests — and ask first only about
  what is ambiguous, destructive, or beyond what the brief said. This is the
  instruction doctrine, not responding's propose-everything-first: the words
  are the human's own, aimed here.
- **The conversation runs in rounds of ordinary Question Sets.** The answers
  to what the human asked lead each Set, so every round reaches their
  devices; questions carry options and recommendations per the guide, and
  the postscript is an ordinary postscript. **The skill says nothing about
  any ending mechanism** — how the follow-up ends is the system's business,
  and the agent needs to know nothing about it.
- **Push each round as work lands**: commit per thing done, push before the
  next ask, so the pull request always shows what has been done and its
  checks run while the human composes.
- The session simply finishes its turn when it has nothing to ask; it never
  exits or wraps anything up itself.

## Acceptance criteria

- [ ] The skill is installed at startup with the others and mounted
      read-only into sandboxes.
- [ ] The prompt builder composes documents, then the follow-up brief last,
      and has unit tests beside the existing builders'.
- [ ] Folding unread deferred answers into a following-up prompt works as it
      does for other dispatched sessions.
