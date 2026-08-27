# 02. Closed is never waiting

## What to build

A closed conversation must never read as waiting on you. Today closing clears
nothing: the stop record and any unanswered Question Sets survive, and the
sidebar's waiting flag excludes only Draft — so a closed conversation keeps its
accent disc, and its header keeps the badge.

The fix lands on both sides, as settled:

- **Write side**: closing a conversation locks its open Question Sets — the
  same lock that already renders as *closed unanswered* on the Timeline. The
  asking sessions are gone for good, so the Sets are no longer answerable.
- **Read side**: the waiting flag and the stop badge additionally exclude the
  Closed state, so the stop record itself stays untouched as history.

Done is deliberately not Closed here: a Done conversation with an unanswered
Set keeps its waiting marks — an answerable ask is still an ask.

## Acceptance criteria

- [ ] A closed conversation shows no sidebar disc and no header badge, whatever
      stop or open Sets it carries
- [ ] Closing locks every open Set on the conversation, and they read *closed
      unanswered* on the record
- [ ] The stop columns are left as they were by closing
- [ ] A Done conversation with an open Set still reads as waiting on you
