# 03. The selection stops at a waiting Set

## What to build

The landing rule CONTEXT.md's **Landing** entry describes. Today the selection
lands on the pending steer, else the last Event with a pane behind it, and
re-runs that landing on every Nudge while the Timeline is advancing — so a
memo, a commit or a notice landing after a Question Set carries the selection
past the Set and leaves it unanswered behind it.

**The rule** becomes: the pending steer, where one stands; else the oldest
Question Set still waiting on the human — its standing *waiting*, whichever of
blocking, disconnected or deferred it is; else the end of the record as today.
A Set answered or locked unanswered is passed over like any other Event. One
function already serves the cold landing and advancing, so the rule changes in
one place and both follow it; the Timeline's scroll-to-bottom is not this and
is untouched.

**A share** carries Sets but has nobody to answer from it, and lands on the
end of its record exactly as it does now.

**Answering puts the Timeline back into advancing.** Any press on a card ends
advancing, and on a phone the Set's pane is reached by tapping its card — so
without this, a phone that answered the Set would stay on the answered Set.
Sending an answer from the Set's pane in the workbench marks the Conversation
as advancing again, and the next landing — the Nudge that follows the answer —
carries the selection on to the next waiting Set or the end of the record. The
standalone sheet a Set is answered on outside the workbench has no selection
and is untouched.

Follow docs/copy.md for any word on screen, and CONTEXT.md's **Landing** entry
for the names: *landing* and *advancing*, never *autofollow* or *follow mode*.

## Acceptance criteria

- [ ] The viewer suite shows a Conversation whose record ends on a memo after
      a waiting Set landing on the Set, moving on to the memo once the Set is
      answered, and landing on the end where every Set is answered or locked;
      a deferred Set waiting stops it too, and the pending steer still lands
      first.
- [ ] A record that grows under an advancing Timeline stays on the waiting Set
      while Events land after it, and moves on once it is answered.
- [ ] Tapping the Set's card and sending an answer from its pane carries the
      selection on to the next thing, and a share with a waiting Set lands on
      the end of its record.
