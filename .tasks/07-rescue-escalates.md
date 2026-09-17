# 07. The Rescue escalates and never stops a session

## What to build

Read the **Rescue** entry of `CONTEXT.md` and the section of
`docs/adr/0018-a-session-says-it-is-done.md` on it. After tasks 01 to 05 no
session is ended on quiet, and the Rescue is the net under a forgotten signal:
it speaks to a session that is **Idle**, with nothing open on the Conversation.
What is left of the old design is its bound — typed **twice at most, never
reset**, and then the session ended where it stands and the Conversation
**Stopped** with a Notice saying it *would not ask*. That bound is a guess about a
session read from outside, and it is what would go on killing a session that is
legitimately waiting. This task replaces it.

**A Rescue the session answers resets the count.** Answered means what the loop
already means by a stir having landed: the session seen at work since the line
was typed (and the echo of the line itself is not that — keep the care the loop
already takes over the echo). The hold-off after a stir stays as it is: a session
just launched, just handed an answer or just spoken to is left until it has said
something, up to the waking ceiling.

**Three in a row unanswered, and Verkstead escalates instead of stopping.** A
Notice on the Timeline and a push to the human's devices — what a stop Verkstead
decided on sends today, without the stop. The Notice says the session has gone
Idle without finishing, asking or declaring a wait, that it was spoken to three
times and did not answer, and what the human can do: type into the Screen, steer,
or press Stop. It carries the evidence a stop's Notice carries, the session's
last output. **The session stays alive and keeps its Worktree**, nothing is
written as stopped, and no driver goes away.

**The card reads *blocked on you*.** That badge is drawn today from a stop or an
open Set; an escalated session is a third thing that raises it, and it goes again
when the session is seen working or is ended. This is durable enough to survive
the viewer reloading, and it is not a stop: Resume is not what the human is
offered, because nothing stopped.

**After escalating, the Rescue holds off** until the session is seen working
again, which also puts the count back to nothing. One escalation per silence: no
second Notice and no second push while the same silence lasts.

**No driver stops a Conversation over *would not ask* any more.** Every caller of
the Rescue loop has an arm that ends the session and writes that stop — the step
driver, inline, instruction, follow-up, the fix session (which ends the session
without a stop). All of those arms go, along with the message; the loop no longer
returns to tell them anything. The long-stop on a backend judged by its screen
stays, and what it leads to is now the ordinary Rescue.

## Acceptance criteria

- [ ] A session that answers a Rescue and later goes Idle again is spoken to again from a count of nothing, any number of times over its life
- [ ] Three Rescues in a row with no answer produce one Notice and one push, and the session is still running afterwards with its Worktree
- [ ] The Conversation is not Stopped by it, its card reads *blocked on you*, and the badge goes when the session is seen working again
- [ ] No further Rescue is typed and no second Notice written while the same silence lasts; work seen after it rearms both
- [ ] No code path ends a session or stops a Conversation because a session *would not ask*
- [ ] The Unix and Windows session suites pass, and the web suite where the badge is drawn
