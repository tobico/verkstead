# 03. The follow-up on a bare branch

## What to build

A Tinker's follow-up runs on a branch that is on no pull request, and two things
assume there is one.

**The skill assumes it throughout.** It opens by saying the branch is already
pushed and already has a pull request open, reads the diff off the pull request,
pushes before every ask because the pull request is what shows the work, and
tells the session not to open one. So the skill learns that a branch may have
none, and says what to do then: read the branch itself rather than a pull
request, commit each thing asked for as before, and carry on — no push to a
branch nothing is tracking, no pull request opened, and nothing said about
checks. What becomes of the branch is still not the session's, and it is still
not told to open anything: Verkstead is what opens the pull request, once the
human says there is nothing else. The rounds, the Sets and `verkstead done` are
unchanged.

**The prompt's opening line promises one.** It tells the session to follow up on
this branch's pull request. On a Tinker that sentence is false, so the opening
says what is true of the branch the session is standing on.

**And a Tinker's follow-up has to survive its session.** The brief a follow-up is
relaunched on is read off the newest Steer into Follow-up — the rounds already
answered are read from the same point — and a Tinker was never steered, so a
Conversation whose session died could be neither resumed nor picked up by the
stall sweep. Where no steer opened the Follow-up, the Brief is what the follow-up
is about: it is read back the same way, and the rounds are read from the move
into Follow-up that the start wrote. A relaunch is otherwise what it always was —
a fresh session on the same subject, primed with what has already been said, with
whatever ask the dead session left standing locked as it starts.

The **Nothing else** box needs nothing: it is drawn on a Set from the
Conversation being in Follow-up, so a Tinker's rounds already carry it. Hold it
to a test rather than assuming it.

## Acceptance criteria

- [ ] A follow-up session on a branch with no pull request commits what it was
      asked for, opens no pull request, and puts the round to the human as an
      ordinary Set carrying the **Nothing else** box.
- [ ] The prompt a Tinker's session is launched on says nothing about a pull
      request the branch does not have.
- [ ] Resume on a Tinker in Follow-up relaunches a session on the Brief and the
      rounds already answered, and a Tinker whose session dies is picked up
      rather than left unrecoverable.
- [ ] A follow-up steered onto a pull request reads its steer's own brief and
      rounds, and its session pushes and is told about the pull request exactly
      as it is today.
