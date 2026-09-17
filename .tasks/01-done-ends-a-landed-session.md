# 01. `verkstead done` ends a session that landed its work

## What to build

Read `docs/adr/0018-a-session-says-it-is-done.md` first, and the **Done signal**,
**Step** and **Rescue** entries of `CONTEXT.md`. They describe the finished work;
this task is its first slice, and the mechanism every later task reuses.

Add the verb and its route, and convert the sessions that are ended today on
*landed plus quiet* — everything the runner sees out through a `Landing`: a
backlog task, the finish step, a roadmap stage's planning, and a grilling's tail
after a pick (the committed backlog, the committed roadmap, the handoff
document).

**The verb.** `verkstead done` takes no arguments. It posts to a new route beside
the other two of the agent contract, under the Conversation the session is
running for — the same scoping `verkstead ask` uses, so the CLI needs nothing new
to find its Conversation. Accepted prints a short confirmation and exits 0.
Refused exits non-zero and prints what is missing on stderr, in words an agent
can act on in the same turn. Asked where no session of the Conversation is
running, it is refused saying there is no session here to end.

**The check.** The server knows which session is running in the Conversation and
what it was sent for. At the signal, and only then, it reads the repository by
the reading that kind already has (`Landing`, and `check`/`landed` in the
runner): the task's box ticked and committed, `.tasks/` taken away, `TODO.md`
arrived, a roadmap this branch wrote, the handoff where it should be. Landed is
accepted; not landed is refused, naming what is missing — *task 3's box is not
ticked and committed*, not *not landed*. For a grilling session the check is
against the **latest pick**: no pick yet is refused saying no Direction has been
picked, and an artifact of a Direction nobody picked does not satisfy it.

**The ending.** An accepted signal is remembered against the session, and the
driver ends the session once it is next **Idle** by the ordinary judgement — so
what the agent prints after the command reaches the Transcript. `landed_and_quiet`
goes: nothing ends one of these sessions on the branch plus quiet any more. A
session that lands its work and then asks a question, or waits on a test run,
stays alive — that is the reported bug. A session that exits by itself without
signalling is read exactly as today: the repository once, landed is done,
anything else stops the run.

**The Rescue beside it.** Today the Rescue leaves alone a session that has landed
its work, because the driver beside it was about to end it. That hold goes for
these sessions: landed and not signalled is exactly a session to speak to. Change
the typed line to offer three moves — carry on with the next step, run
`verkstead done` if the work is finished, or summarize and ask via
`verkstead ask`. Leave its count and its stop alone; task 07 changes those.

**What agents are told.** The skills for these sessions (`next-task`,
`next-stage`, `breaking-down`, `staging`, and the three *after they pick*
branches of `grilling`) say today that Verkstead watches for the artifact and for
the session to go quiet. Rewrite those passages: the last thing the session does,
once the work is committed and everything after it is finished, is run
`verkstead done`, and a refusal says what to put right. Give the Guide
(`verkstead guide`) a section on ending a session, beside the ones on asking.

The session suites fake an agent with a shell script and drive the agent
contract against the Router directly. A stub that is meant to finish now has to
be seen to signal, by whatever means the suite already uses to stand in for the
CLI. Keep the Unix and the Windows session suites both passing.

## Acceptance criteria

- [ ] `verkstead done` from a session whose step has landed is accepted, and the session is ended once it is next Idle; its closing output is in the Transcript
- [ ] A session that lands its step and never signals is not ended by the driver, however long it is quiet, and gets the new three-move Rescue line
- [ ] A signal over a step that has not landed exits non-zero, names what is missing, and leaves the session running
- [ ] A grilling session's signal is refused before any pick, and refused where the artifact is another Direction's than the latest pick
- [ ] A session that lands its work and then opens a blocking Set is left alive while the Set is open and after it is answered
- [ ] A session that exits by itself without signalling is read off the repository as before
- [ ] The five skills and the Guide tell the agent to run `verkstead done`, and nothing in them says a session is ended by going quiet
- [ ] The Unix and Windows session suites pass
