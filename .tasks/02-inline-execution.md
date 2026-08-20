# 02. Handoff document and inline execution

## What to build

The first direction that works end to end. Choosing **implement inline** starts
a fresh session under the Conversation's implementation profile, primed with a
handoff document the grilling session wrote, and that session implements the
work and commits on its own — no gate, no approval, no CLI.

A fresh session rather than the grilling one carrying on, because the two run
under different fixed Profiles (fable and opus today) and a session cannot
change the account it is running as. So everything the grilling session knows
has to be written down before it ends: that is the handoff document, produced as
part of the closing move from task 01.

The handoff is Verkstead's artifact, not the project's, and the sandbox surface
constrains where it can live — a session reaches its Worktree and the Repo's git
directory and nothing else of the machine. So it is written inside the Worktree
at a known path, and Verkstead takes it from there: it belongs on the Timeline
as an Event, and it must never end up in a commit in the human's repo. Getting
it back in front of the implementation session is the other half — the session's
prompt names the skill above the Brief today, and the handoff joins that.

The Conversation moves to Implementing when the session starts.

## Acceptance criteria

- [ ] The grilling session writes a handoff document as part of its closing move
- [ ] Verkstead captures it onto the Timeline and it never lands in a commit in
      the Repo
- [ ] Choosing inline launches a session under the **implementation** profile,
      in the Conversation's Worktree, primed with the handoff
- [ ] The Conversation moves to Implementing with a Moved event
- [ ] The session commits its work without any gate or approval
- [ ] Choosing inline on a Conversation not in Direction is refused
