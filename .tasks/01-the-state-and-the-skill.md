# 01. Investigating, the state and its skill

## What to build

**The vocabulary every task after this one stands on: a `Lifecycle` state called
Investigating, and the skill a session in it runs.** The two halves are one
slice because neither can be shown working alone — nothing can reach the state
until task 02, and nothing runs the skill until then either — and because the
state with no skill behind it would be a state Resume could not honour.

**The state.** A `Lifecycle` variant with a stored word of its own and a reading
for it, a `Moved` Event like every other move, and the viewer's word for it:
*Investigating* on the sidebar row, on the card and on the Timeline's move. Off
the ladder the way Follow-up is — beside it rather than on it — and Closable like
every other state, which needs nothing added because closing is reachable from
everywhere.

Then every table written against the state, because the compiler will name most
of them and the rest are lists rather than matches:

- **The drivers** insist on a driver for it, as they do for the other states the
  work is done in. A Conversation standing in Investigating with nothing driving
  it is a stall.
- **The stall sweep** says what nobody was doing, in its own words, beside the
  other driven states.
- **Stop and Resume** are offered on it, and a launch into it is not one into a
  state nothing drives.
- **The state-to-target mapping** the pending steer's row is drawn back through
  gains its row, Investigating being somewhere a steer can send a Conversation.
- Resume's own exhaustive match gains its arm, which task 02 fills in once there
  is a session to relaunch. Until then it may refuse by name — what it must not
  do is fail to compile or fall through to a state nothing drives.

**The skill.** An `investigating` skill installed beside the others, which is
the following-up skill's shape with the commit obligation inverted: rounds of
ordinary Question Sets, the Preface saying what was found, and an explicit *no
commits, no pushes, no pull request* the way the reviewing and responding skills
already forbid writes before an answer. The Worktree is writable, because
investigation writes probes and runs them — the instruction is the whole of what
keeps commits off the branch, and a commit that lands anyway breaks nothing,
the branch being the Conversation's and going nowhere. The skill is told to
rename the branch through the ordinary naming instruction rather than anything
of its own.

**And the prompt builder for it**, primed with the Brief as the thing to find
out about — the Brief goes in that one place and nowhere else, as a Tinker's
does — plus the rounds already asked and answered where this is a session being
picked up again. It takes its place beside the other builders in the runner's
prompt table.

## Acceptance criteria

- [ ] A Conversation whose stored state is Investigating loads, and the sidebar,
      the card and the Timeline's move all read *Investigating*.
- [ ] The drivers say Investigating is a state something must be driving, and the
      stall sweep names what nobody was doing in words of its own.
- [ ] An `investigating` skill is installed into the sandbox with the others, and
      its text forbids commits, pushes and pull requests.
- [ ] A prompt built for an investigating session names that skill by the path it
      is mounted at and carries the Brief under the heading that says act on it.
