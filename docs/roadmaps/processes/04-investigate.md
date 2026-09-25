# 04. Investigate

## Goal

A draft whose Process is **Investigate** starts into a new state,
**Investigating**: the branch and Worktree are cut as a grill start cuts them,
one session runs under the Implementation Pairing on an `investigating` skill,
and it answers the Brief in rounds of Question Sets — reading, writing and
running whatever code it needs to find things out, committing none of it — and
renames the branch. The human ticks **Nothing else** and the session signals,
and the Conversation is Done; the signal is accepted over uncommitted scratch,
and no pull request is ever asked for. The sidebar and the card read
*Investigating*; the Steer form offers Investigating from every state, taking a
brief the way Follow-up does; Resume and the drivers know the state.

## Decisions in force

- **[ADR-0020](../../adr/0020-a-conversation-has-a-process.md),
  *Investigate***: a Process of its own with a state of its own, a session
  shaped like a follow-up's, the Nothing-else ending, scratch allowed at the
  signal, no pull request, the rename still asked for, and a Steer target from
  every state. Reusing the Follow-up state was considered and rejected because
  the two end differently and the Steer form needs to name them apart.
- **Investigating is a `Lifecycle` variant** with its stored word, its viewer
  word, a `Moved` Event like every move, a row in the exhaustive `resume` match
  and a driver the drivers module insists on. Off the ladder the way Follow-up
  is, and Closable like every state.
- **The skill is the follow-up's shape with the commit obligation inverted**:
  rounds of ordinary Sets, the Preface saying what was found, and an explicit
  *no commits, no pushes, no pull request* the way `reviewing` and `responding`
  already forbid writes before an answer. A writable Worktree because
  investigation writes probes and runs them; the instruction is the whole of
  what keeps commits off the branch, and a commit that lands anyway breaks
  nothing — the branch is the Conversation's and goes nowhere.
- **The Done signal's evidence is the Nothing-else mark**, as a follow-up's is,
  with two differences the rules module carries: uncommitted changes do not
  refuse it, and the session ends with its work rather than on a pull request.
  A dead session on an unmarked round is a stop, as for a follow-up.
- **Ending is Done, directly.** No wrap-up, no watchers, nothing dispatched; the
  Worktree goes with the close as any does, and the branch outlives it.
- **One role, Implementation**, so the Agent control is the dropdown. Readiness
  asks for a brief and that one Pairing.
- **Steer into Investigating** takes a required brief, from every source state,
  making a Worktree where none stands as a steer into any working state does;
  the session it starts is primed with that brief as its Brief. The Steer
  Event's body is the brief, as a follow-up's is.

## Proposed tasks (provisional)

1. **The state** — the `Lifecycle` variant, stored and read, its Moved Event,
   its viewer word, and Resume and the drivers recomputing it. AC: a
   Conversation set Investigating survives a restart and Resume relaunches its
   session; the sidebar reads *Investigating*.
2. **The skill and the prompt** — an `investigating` skill installed with the
   others and a prompt builder priming the Brief, with the naming block
   appended. AC: the session's prompt names the skill and carries the Brief; a
   relaunch carries the rounds already answered.
3. **The start** — Start on an Investigate draft cuts, freezes and lands
   Investigating with the session running. AC: a worktree and branch exist;
   the Timeline shows the move; the Nothing-else box is on its Sets.
4. **The ending** — mark plus signal is accepted over scratch and lands Done
   with nothing dispatched; a signal without the mark is refused with the
   follow-up's words. AC: a dirty Worktree does not refuse the signal; the
   Conversation reads Done; the branch holds no new commits.
5. **Steer** — Investigating on the Steer form from every state, brief
   required, Pairing picked for the Implementation role. AC: a Done Develop
   Conversation steered into Investigating gets a session primed with the
   brief; the Event records the form.
6. **Investigate on the picker** — the row, the dropdown-shaped Agent control,
   readiness on one role. AC: an Investigate draft's Start waits on a brief and
   one Pairing.

## Re-verify at start

- Stages 01 and 02 landed; the role table in the web is where a one-role
  Process is declared.
- `resume.rs` still matches exhaustively on the state and `drivers.rs` still
  lists which states must have a driver.
- The Done-signal evidence enum in `done.rs` still has the follow-up's mark
  variant and the uncommitted-changes check as a separate step, so the
  Investigate case can skip the one and keep the other.
- The Steer form's targets are still the render crate's enum with a runs-in
  check per target and a mapping to `Lifecycle` in the server.
- The Nothing-else box is still drawn by state on the viewer, so a new state
  has to be added to where it is drawn.
