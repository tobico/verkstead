# 06. The rescue, and Resume, for Follow-up

## What to build

Two ways a follow-up that has lost its way gets going again: the rescue,
for a session that is still there, and Resume, for one that is not.

**The rescue.** A running follow-up session that is idle, with no Set open
and the latest settled Set not end-marked (or nothing asked at all), is one
the human can neither answer nor end. After the session grace, a canned line
is typed into the running session — through the same channel a watcher's
keystrokes take — asking it to put what it is waiting on to the human as a
Question Set. **Twice at most**: a session still idle with no Set after the
second rescue stops the Conversation, deliberately, with a Notice saying the
follow-up session would not ask. The canned line's exact wording is settled
here; it should tell the agent plainly that the human sees nothing except
what arrives as a Set.

**Resume.** Resume on a stopped or undriven Follow-up — and a restart, for
stops nobody decided — relaunches a fresh session on the following-up skill,
primed with the steer's brief **plus the rounds already answered**, the way
a relaunched grilling is primed with what the Conversation has settled. A
Worktree the record names and git does not is remade from the branch, as
Resume always makes one. A restart auto-resumes only circumstance stops;
deliberate ones wait for the human, as everywhere.

## Acceptance criteria

- [ ] A follow-up session that goes idle without an open Set and without an
      end mark is typed the canned line after the grace and asks; one still
      idle after two rescues stops the Conversation with a Notice naming
      why.
- [ ] Resume on a stopped Follow-up relaunches the skill primed with the
      brief and every answered round, in the existing Worktree or one
      remade from the branch.
- [ ] A server restart carries a circumstance-stopped Follow-up on unasked
      and leaves a deliberate stop waiting.
