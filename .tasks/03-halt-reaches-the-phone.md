# 03. A halt reaches the phone

## What to build

A deliberate halt that Verkstead decides on fires a web push, the way an
arriving Question Set does — the human answers from a phone, and a silent
stop is found late. The push names the Conversation and what stopped, and
fires once per halt.

Circumstance halts push nothing (restarts will resume them unasked), and
neither will the human's own Stop and Force stop when task 07 adds them —
they were there when they pressed it.

## Acceptance criteria

- [ ] A deliberate halt fires one push naming the Conversation and the stop;
      sweeping past an already-halted Conversation fires none.
- [ ] A circumstance halt fires no push.
- [ ] The stale doc comment claiming interruptions push is gone with the
      behavior made real.
