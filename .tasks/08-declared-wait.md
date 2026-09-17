# 08. `verkstead waiting` declares a background wait

## What to build

Read the **Declared wait** and **Idle** entries of `CONTEXT.md` and the section
of `docs/adr/0018-a-session-says-it-is-done.md` on a declared wait. An agent that
starts a build or a test run in the background and ends its turn is Idle by every
reading Verkstead has, and after task 07 that gets it spoken to three times and
the human's phone buzzed. This verb is how it says so first.

**The verb.** `verkstead waiting [LENGTH]`, a second verb beside `done`, posting
to a route of its own under the Conversation. The length is how long the agent
expects to wait — `45m`, `90s`, `1h`. **At most one hour; fifteen minutes where
none is given.** A longer one is refused, exits non-zero and names the maximum,
and so is one that does not parse. It returns at once: the name is a statement
because `verkstead ask` blocks and this must not read as one more thing that
does. Asked where no session is running, it is refused like `done` is. Declaring
again replaces the wait standing, which is how a wait is renewed.

**A waiting session is active, not Idle.** Make this one change in the one
judgement everything reads, rather than teaching each reader about waits: while a
wait stands, the session reads as at work to the sidebar card and the Timeline
row (the spinner, not the Idle mark), to the Rescue (never spoken to), and to the
long-stop on a backend judged by its screen.

**When a wait is over:**

- its time runs out — the session is then Idle if its backend says so, and the
  Rescue's grace starts from there;
- the session is next seen working *after it has gone quiet behind the
  declaration*. The turn that declares the wait goes on printing for a moment —
  that is not the background work coming back. The wait is released by work seen
  once the session would have been Idle but for the wait;
- `verkstead done` is accepted. A signal clears a wait, so an accepted done still
  ends the session once it is next Idle.

An open blocking Set already holds the Rescue off and needs no wait declared;
nothing about that changes.

**What agents are told.** Add a section to the Guide on waiting in the
background: before ending a turn with work of your own still running, run
`verkstead waiting` with how long you expect it to take; declare again if it runs
over; an ask needs none. Add the third sentence to the Rescue line — if
background work you started is still running, run `verkstead waiting` and keep
waiting. And put one short paragraph pointing at the Guide's section in every
skill under the server's skills directory, since any of those sessions may run a
build.

## Acceptance criteria

- [ ] `verkstead waiting 45m` is accepted and returns at once; a bare one stands for fifteen minutes; more than an hour, or a length that does not parse, is refused naming the maximum
- [ ] A session with a wait standing shows as working on its card and its Timeline row, and is never spoken to by the Rescue, however quiet it is
- [ ] The output of the declaring turn does not release the wait; work seen after the session has gone quiet behind it does
- [ ] When the time runs out with the session still quiet, the Rescue speaks to it after its ordinary grace
- [ ] An accepted `verkstead done` clears a standing wait, and the session is ended once Idle
- [ ] The Guide, the Rescue line and every skill mention declaring a wait
- [ ] The Unix and Windows session suites pass, and the web suite where the mark is drawn
