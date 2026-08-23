# 02. The idle mark

## What to build

Replace the "running" text badge on the agent-output Timeline item and the
details pane with the mark the sidebar's conversation card already uses: a
slowly turning ring at the right edge while the session works, and an
**empty (not filled) circle** while the session is running but idle.

Idle is the server's judgement, not the page's: a session is idle when its
quiet clock — no terminal output — has passed **3 seconds**. Claude Code
repaints a spinner continuously while it works, so quiet means idle fast;
the case this exists for is a grilling sitting on a blocking ask for hours
while the Timeline says it is busy. The agent-output event on the wire says
whether the writing session is idle beside whether it is running.

The page only re-reads on Nudges, and a session going quiet is exactly when
it stops producing them — so the relay announces the crossing *into* idle on
the existing conversation nudge kind (which also reaches the sidebar list,
for task 03). Waking needs no announcement: the session's next output already
makes open pages re-read the Conversation, which puts the mark back to
turning. A page loaded mid-idle is right immediately, because the flag is
computed on every read.

The mark replaces the badge in both places the event is drawn — the Timeline
item and the details pane's summary line above the switcher. Reduced motion
keeps the still ring, exactly as the sidebar card's mark does today.

## Acceptance criteria

- [ ] A working session's Timeline item and details pane show the turning
      ring at the right edge; the "running" text badge is gone from both.
- [ ] A session quiet for 3 seconds flips to the empty circle on open pages
      within moments, with no reload — and flips back when it speaks.
- [ ] A page opened while the session is already idle shows the empty circle
      straight away.
- [ ] `prefers-reduced-motion` holds the ring still; a finished session shows
      no mark at all.
