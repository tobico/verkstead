# 02. Uncommitted changes are asked about first

## What to build

Task 01 has a take-up close a Done holder, and a close removes the worktree by
force: whatever was uncommitted in it is discarded. The human is asked about
that before it happens, and about nothing else — see
[ADR-0020](../docs/adr/0020-a-conversation-has-a-process.md), *One open
Conversation per pull request*.

**Asked only where there is something to lose.** A holder whose worktree is
clean is closed with no question, exactly as task 01 left it. A holder whose
worktree — or a companion checkout it may write in — holds uncommitted changes
stops the press: modified, staged, or untracked and not ignored, which is the
reading `verkstead done` already takes of a worktree before it accepts a signal.
A worktree whose directory has gone holds nothing.

**The press stops with a named outcome** carrying the Conversations that would
lose something, each by id and by the branch it goes under. Nothing is closed
and nothing is made; the draft stays a draft. The composer draws it under the
press the way it draws a refusal, each Conversation a link, and the press then
reads as going ahead. The second press says so to the server, which closes and
takes up.

**The confirmation is for what was named.** The server reads the worktrees again
on the second press rather than trusting the first: where a Conversation that
was not named now holds uncommitted changes, the press stops again naming it.
A list rather than one, because task 03 adds a stack's neighbours to it.

The compose page's create replay carries take-up outcomes to the draft it made;
it carries this one too.

## Acceptance criteria

- [ ] A Done holder with a clean worktree is closed and taken up from with one
      press
- [ ] A Done holder with a modified, a staged or an untracked file stops the
      press naming it, and the holder is still Done with its worktree and its
      changes
- [ ] Changes in a companion checkout the holder may write in stop it too
- [ ] The second press closes the holder and takes the pull request up
- [ ] A second press confirming a list that is no longer the whole of it stops
      again with the new list
- [ ] The composer draws the named Conversations as links under the press, and
      the press that follows is the confirming one
