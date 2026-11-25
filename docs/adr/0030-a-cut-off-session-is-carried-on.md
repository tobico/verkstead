# A cut-off session is carried on

Generalises the **Carried Conversation** of
[ADR-0020](0020-cluster-mode.md)'s *Transfer*, which is where a session ended
mid-turn was first picked up by the harness's own resume rather than replaced by
a fresh one, and joins the two occasions already borrowed from it:
[ADR-0028](0028-adding-from-an-answer.md), *The carrying-on is the Carried
Conversation's, on this device*, where an answer that widens the sandbox ends the
session waiting on it; and [ADR-0029](0029-work-is-placed-by-platform.md), where
*Keep it here* ends the session that asked to move the work. All three stand
where they stood. What this adds is the occasion none of them is: a session that
nobody ended on purpose.

The incident it was written for. Conversation 25 ran a Tinker under a Profile
bound on another device, so this annex ran it from an **Account mirror**; the
session asked a Question Set and sat idle; the other device refreshed that login
overnight and spent the refresh token the mirror's copy held; the human answered
at half past midnight, the session's own refresh failed, and the run stopped.
**Resume** was pressed, and what started was not the follow-up reading the three
answers: it was a fresh session that recomputed the job, decided the follow-up
was over and its work was on no pull request, and went to open one. The human
got the conversation back by typing `/resume` at the **Screen** by hand. Two
things had gone wrong at once — the agent's own thinking was thrown away, and the
job changed under it — and the second is the worse of the two.

## The decision

**A session cut off mid-turn is carried on; one that finished its step is not.**
*Cut off* means the session stopped before its step was done, whatever stopped
it. One whose step was done is left to Verkstead's own **Resume** exactly as it
was, because what comes after a finished step is a fresh session however anybody
decides it — there is no conversation to lose.

Each of the following was decided while this was built, and the alternative
named is what was turned down.

- **The occasion is read at the launch rather than written at the stop.** The
  other three occasions are each somebody's act, with something running to write
  the `continued_sessions` row as it goes. A stop is nobody's answer, and two of
  the ways a session is cut off leave nothing running to write anything: a crash,
  and a server killed or shut down, there being no graceful path out of this
  process. So the reading sits in the recompute that every way back in passes
  through — the human's press, a login's own carry-on after a sign-out, a
  restart's startup pass, and an arrival — and writes the row there for the
  launch after it to spend. Writing a row at each stop was turned down: it
  answers the stops that have something running to write one, and leaves the
  crash and the kill with nothing — and a shutdown hook for those is a promise a
  `SIGKILL` breaks. Which is also why a restart and a crash cost this nothing:
  there was never anything to run at either.

- **Mid-turn is the state's own reading, not a flag.** A **grilling**'s is
  unconditional — picking a direction moves the Conversation out of Grilling, so
  one still grilling with a session behind it was cut off whatever stopped it. A
  **run**'s is the state of the task it was building, read off `.tasks/` and the
  branch by the same reading the launch itself makes of them: every step of a run
  ends in a commit, so a session that left its work standing was part way through
  and one that got its commit in stopped between two steps. A **follow-up**'s is
  how its session ended, a round being a conversation with nothing of it on the
  branch: a session taken away while it was running and one that fell over where
  it stood were each mid-sentence, and one that exited cleanly had the turn it
  was having. Reading the stop's own kind instead was turned down — a force stop
  over a run that had just committed is not a conversation anybody is losing, and
  a crash over one that had is not either.

- **An inline implementation and a roadmap's own writing read as between steps.**
  Neither has a backlog to have been part way through, and the question each of
  them does answer — whether the branch is already on a pull request — is asked
  over the network by the launch itself and may come out as a wrap-up with no
  session in it at all. A row written in front of that is a row nothing spends,
  which the next session of the work would take up and be told to resume a
  conversation that had nothing to do with it. Carrying those on is a row written
  where the session is launched rather than in the recompute, and was left for
  whoever wants it.

- **The job must not change under a carry-on.** A launch with a conversation
  standing to be carried on *is* the session that was cut off, resumed, so what
  it works is its own turn's to decide. The drivers that re-derive a job before
  launching ask whether one is standing before they re-derive it: a follow-up is
  picked up as that follow-up rather than diverted into opening a pull request,
  and a backlog with nothing left in it is carried on rather than decided again.
  This is the incident's own fix, and it is the half that was not about losing
  context: a recompute is the right answer for a fresh session and the wrong
  question entirely for a resumed one.

- **Every way a session is cut off, by that one reading.** A sign-out stop, a
  sign-out on a login replaced since, a **Force stop**, an **Agent Profile** out
  of usage window, a stall found over a session that fell over, a harness that
  exited or crashed mid-turn, and a restart, a kill or a shutdown over a session
  that was running. The plain **Stop** needs no rule of its own: it lets the step
  it found reach its own end, so the session it stops is one whose turn was over
  and the reading says *between steps* about it by itself. A **Steer** never
  reaches the reading at all — the human saying what is to happen instead
  replaces that session, which is ADR-0020's rule unchanged.

- **A stop waiting for a press keeps its cut-off until the press.** A restart
  leaves a stop somebody decided on exactly where it stands, so it reads nothing
  and writes nothing down, and the force stop answered the next morning finds
  the session that stopped as it stood. Which falls out of reading at the launch
  rather than at the stop: the question is asked where a session is about to
  start, so a Conversation nothing is starting for has nothing written against
  it.

- **The note is one sentence, and says nothing about why it stopped.** The stop
  wrote its own **Notice** at the time, and a second telling is the human reading
  the same bad news twice. The **Question Sets** the session was idling on are
  named under the ids they already have — nothing moved, so nothing was
  renumbered — with the `verkstead answers` line that fetches the Answers, and
  they are left **open** rather than locked, as the second and third occasions
  leave theirs: the reader is the one coming back. No note at all was turned down
  for a mechanical reason that is worth writing down — every harness takes the
  prompt as its first turn, so an empty note is a resumed agent sitting at its
  prompt until the stall sweep stops it.

- **And the Timeline says which of the two it was**, in a clause of the cut-off's
  own: the session that was resumed, or the reason it was not. Four of
  ADR-0020's five reasons still part the two outcomes. No session on the record
  is one of them here, read by the launch itself rather than known at an arrival
  — a session that really ran under a backend that names none, and one whose own
  id was never written down — and a Pairing on another harness, a record that is
  not where the harness will look for it, and the harness refusing are the other
  three. The memory switch is not one here, for ADR-0028's reason: nothing
  moved, so the Conversation's own home holds the log either way. And a
  Conversation no session ever ran for says nothing at all, there having been
  nothing to cut off.

- **And the account readers begin again where the resumed session does.** A
  harness that resumes a conversation draws it again, so the sign-out line or the
  exhausted-window line the run stopped on the first time lands back on the
  terminal, the frame and the **Capture** of a session whose account is perfectly
  good. Every reader that can stop a run opens where this session does: what it
  has printed is empty, the frame is the one it draws, and the log is followed
  from the length the launch measured before it started. The log is the one of
  the three that needed it, being the one record that outlives a session.

## What this does not change

The three occasions above keep their own rows, their own notes and their own
clauses on the Timeline, and the guard that the cut-off reading writes nothing
where a row already stands is what keeps them theirs. Nothing about a stop
changes: it is recorded the same way, marked the same way, and reaches a phone
or does not by the same rule — what changes is what the press after it starts.
A Conversation that finished its step still gets Verkstead's own Resume, which
is a complete way to take work up. So does a state with no cut-off reading in
it: **Wrapping**, whose press starts the five watchers rather than a session,
and **Investigating**, which this did not reach — an investigation cut off
mid-round is still re-primed from the question it was opened with and the rounds
already answered. And nothing here is the human's to answer: both outcomes are a
session appearing on the Timeline, and the clause beside it is there to say
which, not to ask anything.

Two neighbours of the incident were deliberately left out, and both are easier
with this in place: a mirrored sign-out carrying on by itself where the bound
device's login has been replaced since, and a **Notice** that names *dragon*
where two devices are called that.
