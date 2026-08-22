# 04. The Hold

## What to build

Typing into a live Screen, and what that costs Verkstead.

Keystrokes go up the socket to the session's terminal. The first one takes the
Hold — the human is at the keyboard, and Verkstead is not.

While a Hold lasts Verkstead records and nothing else. Capture, transcript
tailing and Timeline updates carry on; ending the session and advancing the run
are suspended. That is a gate every driver asks rather than one flag in one
place, because each of them ends or advances something on its own: a backlog
step ended on landed-plus-quiet, a fix session ended on committed-plus-quiet, a
review session ended once it has asked, and the wrap-up that starts the next
roadmap stage.

The Hold ends **only** by an explicit hand-back control in the workbench. Not
by a timeout, not by the socket dropping, not by the tab closing — resuming
over a half-finished intervention is worse than a stalled run. A session that
exits while held waits for hand-back, and hand-back then runs the ordinary
end-of-session evaluation on whatever the human left: the Step's commit landed
so the run goes on, or it did not so an Interruption.

A held Conversation carries *blocked on you*, pointing at the held session's
own Timeline Event so the badge has somewhere to go. The Hold itself leaves no
Event — the Timeline records the work, not the watching.

Aborting the Conversation still ends the session, held or not. That is the
human answering rather than Verkstead advancing.

Quiet-detection stays keyed on what the terminal prints; keystrokes do not feed
it. What protects a human mid-typing is the Hold suspending session-end, not
the clock.

## Acceptance criteria

- [ ] Typing in the Screen reaches the session, and the first keystroke puts
      the Conversation in a Hold carrying *blocked on you*.
- [ ] A held Step session is never ended by quiet, however long it stays quiet,
      and nothing advances behind it.
- [ ] A session that exits while held advances nothing until hand-back, and
      hand-back then judges it the ordinary way — commit landed, the run goes
      on; nothing landed, an Interruption.
- [ ] Dropping the socket, closing the tab and restarting the browser each
      leave the Hold where it was, and no Hold leaves an Event on the Timeline.
