# The pick informs the agent; artifacts move the machine

Choosing a direction used to be two presses and a beheading: accepting the
wrap-up proposal ended the grilling session on the spot, and the human walked
back to the Timeline to press a separate chooser, which launched a fresh agent
to rebuild the plan from a handoff. The context that had settled the work was
gone at exactly the moment it was worth the most — the fresh breakdown agent
wrote the backlog from a summary of a conversation it never had.

Now the chooser rides the closing Set, and a pick is delivered back to the
still-living grilling session rather than acted on. Nothing state-changing
happens on the answer: the session judges from the whole Response whether
everything is clear, and *proceeding is producing the picked direction's
artifact* — the committed backlog, the committed roadmap, or (for inline) the
handoff document, each ended on artifact plus quiet like any Step. The
Direction state leaves the ladder, planning work stays with the grilling
context under the Grilling Profile, and the handoff is written after the
choice, for inline alone — the one direction whose builder is a fresh session.

## Considered Options

- **Hard routing on the answer** (the previous design, minus the second
  press): a pick immediately ends the grilling session and launches the
  direction's fresh session. Simple and immediate, but it forfeits the
  grilling context for planning work, forces the handoff to be written
  *before* the direction is known — rewritten on every refused round, and
  wasted entirely when the artifact is a backlog the same context could have
  written — and leaves no room for the human's pick-side feedback to reach
  anyone who can act on it.
- **An explicit proceed signal**: the agent runs a CLI verb to declare it is
  going ahead. Unambiguous, but it is a second report beside the artifact and
  a forgettable one — the class of half-made report the Step doctrine exists
  to avoid. The artifact already says everything the signal would.
- **Keeping the standalone chooser** with only the session kept alive:
  preserves the "acceptance settles understanding, direction settles later"
  separation, but that separation was the two-step problem itself, and the
  dwell state it requires (Direction) is a rung nothing needs to wait on once
  the pick can ride the Set.

## Consequences

- Acceptance is soft: a pick *lets* the agent proceed and never *makes* it.
  The agent may come back with another Set or another proposal; a later pick
  supersedes, latest wins. The direction is still never the agent's to
  change — it proceeds on the picked direction or argues by proposing again.
- The state machine keys off session lifecycle, not answers: a Conversation
  is Grilling until its grilling session ends, Implementing when the
  Implementation Profile drives, and a roadmap Conversation skips
  Implementing entirely (its building belongs to its Stages).
- The handoff exists only where a context boundary is actually crossed
  (inline). Task-list and roadmap plans are written by the grilling context
  and recorded as the committed artifact; downstream sessions read the repo,
  not a summary.
- A retried planning tail runs fresh with no handoff — Brief plus retry note.
  Accepted: the retry path is rare, and the stalled handler owns the failure
  surface.
- No compatibility path for Conversations mid-old-flow: the migration moves
  a Conversation caught in the retired Direction state to Aborted.
