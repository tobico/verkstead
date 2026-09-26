# 02. The Investigate start

## What to build

**Investigate becomes a Process a human can pick, and Start on one lands
Investigating with a session running.** Both halves in one slice for the reason
Tinker's were: a row offered over a press that grills would start the wrong
thing, and a start path nothing can reach is not a start.

**On the composer.** The Process picker offers *Investigate*, and the server
accepts it — both lists grow a row, the viewer's and the record's. The
per-Process role table already says Investigate is run under the Implementation
role alone with the **Agent** control taking the flat dropdown shape, so nothing
about the control needs adding: the picker stands in the setup row where a
trigger would have stood, and the row's own label names it.

**Readiness asks for a brief and that one Pairing.** No grilling Pairing and no
Review Pairing are waited on, there being no picker drawn for either, and the
inert Start says it is waiting on *one role*, which the role table already counts.
The reading that decides the press is the reading that decides the button, as it
is for every other Process.

**The press does the grill start's own sequence.** Fetch, resolve the base, cut
the branch, make the Worktree and every companion's, clear whatever task list the
base was carrying, freeze the Brief and the Pairings, remember them against the
Repo — all the same work, refused by the same names, with the branch name settled
the same way. What differs is two things: the Conversation lands
**Investigating**, and the session launched into the Worktree is the
investigating one. A Conversation that already has a Worktree is handled exactly
as it is for a grill start.

**The session runs in rounds, driven for as long as it runs.** One session under
the Implementation Pairing on the `investigating` skill, primed with the Brief,
with the naming instruction appended as it is on every first session. The
registration is handed on to the run that drives the rounds rather than dropped
at the end of the press, because that run is a loop rather than a launch. A
session that goes idle with nothing put to the human is spoken to, and one that
is gone on an unmarked round is a stop, exactly as for a follow-up.

**The Nothing-else box is drawn on its Sets.** The box is drawn off the state the
Conversation is in, so Investigating joins Follow-up as a state whose Sets carry
it. Its helper text is worded by state: an investigation ends rather than wraps
up.

**And a lost session is picked up again.** A press of Resume, and a restart's
sweep, relaunch the investigating session on the Brief and on every round already
asked and answered inside this Investigating — read from the newest way *into* the
state downwards, as a follow-up's are, so a Conversation investigated twice does
not reopen the first round's questions. There is no owed-pull-request branch to
worry about here: an investigation never ends on one.

## Acceptance criteria

- [ ] A Draft can be set to Investigate, and its setup row draws one **Agent**
      dropdown with no Grilling and no Review picker.
- [ ] Start is inert while the Brief is empty or the Implementation Pairing is
      unanswered, saying it waits on a brief and one role.
- [ ] Pressing a ready Investigate draft's Start leaves the Conversation in
      Investigating with a branch, a Worktree and every companion checked out,
      the Brief and the Pairing frozen, and the move on the Timeline.
- [ ] One session runs under the Implementation Pairing on the `investigating`
      skill, its prompt carrying the Brief and the instruction to rename the
      branch; its Sets carry the Nothing-else box.
- [ ] Killing that session and pressing Resume starts another on the same Brief,
      primed with the rounds already answered rather than asking them again.
