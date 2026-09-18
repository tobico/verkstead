# A session says it is done

Supersedes one option [ADR-0008](0008-pick-informs-artifacts-move.md) rejected,
and the rule the runner was built on: that a session reports through the
repository alone, and Verkstead ends it on what landed plus quiet.

Every session Verkstead launches is an interactive agent, which idles when its
work is over rather than exiting. So Verkstead ends them, and until now it
decided when by watching from outside. A backlog step, a grilling's artifact, an
inline run, an instruction and a fix were each ended once the repository showed
the work **and** the session had printed nothing for a five second grace. A
review or a batch of comments, which reports through nothing but its own words,
was ended on a minute of quiet with no Set of its own open. And a session idle
with nothing open and nothing landed was spoken to twice by the **Rescue** and
then stopped as one that *would not ask*.

All three are guesses, and an agent that waits on something in the background
defeats all three. It starts a build, a test run or a wait of its own, ends its
turn, and sits there idle until the harness wakes it — which from outside is the
shape of a session that has finished, or one that is stuck. The human reported
both halves of that:

- A grilling session ended while it was waiting on an answer. The wait that ends
  a grilling was *artifact landed plus quiet*, and it never looked for an open
  Set. A roadmap on the branch was enough.
- An implementation session ended with uncommitted work while it waited for a
  test run. An inline run's *landed* is any new commit, so one that committed a
  first slice and then waited on its tests was ended five seconds later.

Only an open blocking Set held the enders off, and only some of them.

## The decision

**A session is ended because it said it was done.** The agent runs
`verkstead done`, and that signal is the one thing that ends a session on
purpose. It holds for every kind of session and every backend. Nothing on the
branch ends a session any more, and neither does quiet.

**The repository is still read, at that moment and only then.** What *done*
means is unchanged and is still the kind's own: a task's box ticked and
committed, the finish step's `.tasks/` taken away, the picked Direction's
artifact for a grilling, a new commit for an inline run or an instruction, the
human's own *Nothing else* mark for a follow-up. What changed is what it is for.
It used to be the trigger; now it is the check on a trigger the agent pulls.

**A signal the evidence does not bear out is refused.** The command exits
non-zero and says what is missing, the session stays alive, and the agent puts
it right in the same turn. It is refused when:

- the work has not landed, by the kind's own reading above — a grilling with no
  pick, or with the artifact of a Direction nobody picked, included;
- the Worktree, or a companion repo the Conversation may write in, holds
  uncommitted changes, the refusal naming the files;
- the session is one meant to end on a pull request — a finish step, an inline
  run, a roadmap's own session, the session sent to open one — and the branch
  has none open, or a companion repo the work committed in has none. The finish
  sequence covers each companion in that repository's own words, so each is a
  pull request a session can stop short of, and the wrap-up stops the run over
  one a session later with whoever could have opened it already gone. The
  companions are read the one way both of them read them, so the signal and the
  wrap-up cannot come to disagree about one repository. GitHub out of reach
  reads as accepted, because a session is not to be held hostage by somebody
  else's outage.

This is the answer to what ADR-0008 held against an explicit signal, that it was
"a second report beside the artifact and a forgettable one". A half-made report
is now sent back rather than acted on, and a forgotten one is what the Rescue
below is for.

**An inline run sent onto work already built is checked against its pull
request instead.** A run whose first session committed the work and went before
the push is picked up by a second, and what that one is sent for is to read the
branch over and carry it to a pull request: it has nothing to commit, and a rule
that demanded a commit would leave it signalling into a refusal it could never
put right. It is one of the sessions that end on a pull request, so that is what
it is asked for. The first session on a branch holding nothing is asked for its
commit as before.

**A fix session is the one kind with nothing to show.** One that finds nothing
to commit is not refused for it: what a fix is judged by is the check on GitHub,
asked again once the session is over, and the wrap-up's two goes at a check are
the bound it already has. A rule that demanded a commit would leave a fix session
with nothing to fix unable to end.

**The signal is a bare verb.** Verkstead knows which session is running in the
Worktree and what it was sent for, so there is nothing for an argument to say.
*I cannot finish* is a `verkstead ask`, not a kind of done.

**A blocking Set of the session's own still open does not refuse it.** The
session has said it is finished, so nothing will read the Answer: the Set is
locked unanswered, exactly as a relaunch locks one a dead session left hanging.
A Deferred Ask is left standing, nothing having ever waited on one.

**An accepted signal ends the session once it is next Idle**, by the ordinary
judgement, so the closing words an agent prints after its last command reach the
Transcript.

**A session that exits by itself without signalling is read as it was.** The
repository is asked once; a landed step is a step done, and anything else stops
the run.

### A declared wait

`verkstead waiting 45m` says the session is about to end its turn with work of
its own still running in the background. The agent names the length: at most an
hour, and fifteen minutes where it names none. A wait can always be declared
again, so the maximum bounds only how long a wait whose background task died
sits before anybody asks.

**A waiting session is active, not Idle** — one judgement, read by everything
that reads Idle, so it shows the spinner on its card and its Timeline row and is
never spoken to by the Rescue. The wait is over when its time runs out, when the
session is next seen working after it went idle, or on `verkstead done`.

The name is a statement rather than a command. `verkstead ask` blocks, and
`verkstead wait` would read as one more thing that does; `verkstead waiting`
returns at once.

### The Rescue escalates, and never stops a session

An Idle session with nothing open and no wait declared is still spoken to: that
much of the Rescue stands, and it is now the net under a forgotten signal. The
line offers three moves — carry on, run `verkstead done` if the work is
finished, or put where it has got to to the human as a Set — and says to declare
a wait where background work is still running.

**A Rescue the session answers resets the count, and unanswered ones never stop
a session.** After three in a row with no answer Verkstead escalates
instead: a Notice on the Timeline and a push to the human's devices, which is
what a stop sends, without the stop. The session stays alive, the card reads
*blocked on you*, and the Rescue holds off until the session is seen working
again. What happens next is the human's — type into the Screen, steer, or press
Stop.

The old bound — twice, then ended where it stands — is what would have gone on
killing a long background wait under any signal, and *would not ask* was always
a guess about a session read from outside it.

## Considered Options

- **A waiting signal alone**, quiet still ending sessions. The smallest change:
  a declared wait becomes one more thing that counts as open. Rejected because
  of what forgetting costs. A forgotten wait is a session ended mid-work with
  what it had uncommitted, which is the failure this began with; a forgotten
  done costs one Rescue and one short turn.
- **Better guessing, no new verb**: reading the session's process tree and
  taking a live background command as not idle. Nothing to forget, but it is
  right only where the wait is a child process, a dev server left running never
  ends, and it differs per backend and per platform — a fourth guess rather than
  the end of guessing.
- **Accepting a signal the evidence contradicts** and stopping the run with a
  Notice, or trusting it outright. The first turns an agent's slip into the
  human's problem when the agent could have fixed it in the same turn; the
  second is the half-made report ADR-0008 was right to fear.
- **Rescues on a lengthening interval** with a stop after an hour, in place of
  a second verb. No second thing to forget, but every long wait costs wasted
  turns, and the ceiling is one more number that is wrong for somebody's build.
- **A fixed ceiling on a wait**, or none. The agent is the one that knows how
  long its build takes, and a wait with no end is a session that sits for ever
  once its background task dies without waking it.
- **An outcome argument or a summary on the signal.** Nothing reads one: the
  repository says what landed, and the Transcript holds what the session said.

## Consequences

- ADR-0008's headline stands — *the pick informs the agent* — and its second
  half is revised: an artifact no longer moves the machine by landing. It is
  what the grilling session's done signal is checked against, and the latest
  pick is still the one that counts.
- The Step doctrine's *a commit is the one report an agent cannot half make* is
  still why the evidence is a commit. It is no longer why a session ends.
- `Pace::proposing` stops being an ender's grace, and the rule about a session
  that never said a word goes with it: no session is taken at its silence any
  more, so there is no silence to tell from a review that found nothing.
- The session sent to open a pull request after a run stopped short of its push
  stays, as the net under a session that exits by itself. One that signals is
  refused until the pull request is open, so it should be rare.
- Every skill ends by saying to run `verkstead done`, and the Guide carries the
  two verbs beside `ask`. The skills ship inside the binary, so there is no skew
  to carry between a server that waits for a signal and a skill that never
  mentions one.
- A session nobody answers for sits, holding its Worktree, until the human
  looks. That is chosen: the escalation reaches a phone, and a session ended by
  a guess loses work where one left alone loses only time.
