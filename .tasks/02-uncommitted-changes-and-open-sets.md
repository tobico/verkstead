# 02. Done is refused over uncommitted changes, and locks an open Set

## What to build

Two more rules on the done signal task 01 built, both of them the signal's own
rather than any one kind of session's, so they belong in the shared check and
hold for every kind the later tasks convert. See
`docs/adr/0018-a-session-says-it-is-done.md`.

**Uncommitted changes refuse the signal.** One of the two failures this work
began with was a session ended with uncommitted work. So a signal given while
the Worktree holds uncommitted changes — modified, staged, or untracked and not
ignored — is refused, and the refusal names the files (a long list cut short,
saying how many more). The agent commits them or discards them and signals
again. The same holds for **every companion repo the Conversation may write
in**, each named in the refusal by the label the Diff already gives it; a
companion the Conversation can only read is not looked at. Read git the way the
runner already does — without taking optional locks — because the session may be
committing at that moment. A repository that will not answer reads as *refused*,
saying so: the session is alive and can simply signal again.

**A blocking Set of the session's own still open does not refuse it.** The
session has said it is finished, so nothing will ever read the Answer. On an
accepted signal, lock the Set unanswered — the same locking a grilling relaunch
does to a Blocking Ask a dead session left hanging, so the human sees one kind of
locked Set. A store-and-nudge Set the session is standing behind is locked the
same way. A **Deferred Ask is left standing**: nothing ever waited on one, and
its Answers reach a later session by design. Whose Set it is is read the way the
runner already reads it — every Set that landed after the session's own Event.

Say both rules in the Guide's section on ending a session: commit or discard
before signalling, and do not signal with a question you still want answered.

## Acceptance criteria

- [ ] A signal with modified, staged or untracked files in the Worktree is refused and names them; after a commit the same signal is accepted
- [ ] A signal with uncommitted changes in a writable companion repo is refused naming the repo and the files, and one in a read-only companion is not looked at
- [ ] Ignored files do not refuse a signal
- [ ] An accepted signal locks the session's own open blocking Set unanswered, and the Timeline shows it as any other locked Set
- [ ] An accepted signal leaves a Deferred Ask open and answerable
- [ ] The Unix and Windows session suites pass
