# 05. Follow-up sessions

## What to build

Convert the **follow-up** session — the following-up skill, a conversation in
rounds of Question Sets about work already on a pull request — to the done
signal. It is the last kind still ended on quiet. See
`docs/adr/0018-a-session-says-it-is-done.md` and the follow-up entry of
`CONTEXT.md`.

Today what ends a follow-up is the human's mark and the session's silence
together: the newest round they answered carries **Nothing else**, nothing is
open on the Conversation, and the session has printed nothing for the grace
(`nothing_else_and_quiet`). The mark stays the human's to give. What changes is
the other half: the session, having seen the mark come back with a Response, runs
`verkstead done`.

**The signal is checked against the mark.** A signal while the newest answered
round does not carry Nothing else is refused, saying the human has not said there
is nothing else and that the next round goes to them as a Set. Whether there is
anything else is theirs to say; a follow-up session cannot end itself. With the
mark, the signal is accepted under task 02's rules and the session is ended once
it is next Idle. `nothing_else_and_quiet` goes. What follows the ending is
unchanged: the Conversation is Wrapping again over its pull request, with the
checks put back to waiting where the follow-up pushed anything.

The mark alone still ends nothing — it never did, because the work the last round
asked for comes after it — and now quiet does not either, so a follow-up that
pushes and then waits on its checks in the background stays alive.

Rewrite the ending passage of the `following-up` skill: when a Response carries
the Nothing-else mark, finish and push what that round asked for, and then run
`verkstead done`.

## Acceptance criteria

- [ ] A follow-up's signal without the Nothing-else mark on the newest answered round is refused, and says why
- [ ] With the mark, the signal is accepted and the session is ended once Idle; the Conversation returns to Wrapping as before
- [ ] A follow-up with the mark that never signals is not ended by the driver, and is spoken to by the Rescue
- [ ] The `following-up` skill tells the agent when to run `verkstead done`
- [ ] The Unix and Windows session suites pass
