# 05. OpenCode, and the row in the carried database

## What to build

The last harness, and the one whose store is a database rather than a tree.

**Its id is the store's own**, as Codex's is. OpenCode takes no session id at
launch — its session flag means *continue this one* and is validated against
the store before the interface starts, so a fresh name would be a session that
never starts — and the reader that follows its records already finds the row
this session wrote: the one recording this Worktree that was created after the
session was launched. That row carries the id. It goes onto the record the same
way Codex's does, beside the session it belongs to, and task 01's lookup takes
it from there.

**Its resume is the flag and the prompt flag**, the shortest line here staying
the shortest: the session flag with the id, the prompt under its own flag as it
already is, the model and the approvals unchanged.

**And the directory the row is keyed by is the sending device's.** The memory
sync carries the whole database — the one file and the two siblings SQLite
keeps beside it — because narrowing further means writing rows of a schema that
is opencode's own and moves between releases. What travels with it is the
working directory recorded against every session in it, this Conversation's
included, and the validation the resume flag does is against a store that has
to agree where the work is. So the row for the session about to be resumed is
brought onto this device's Worktree path as the database lands.

That is the dependency the records reader deliberately does not take — it is
allowed to stop reading a schema that moved, where this has to keep writing. So
it is written to fail the way everything else here fails: a database whose
shape this build does not recognise leaves the row alone and the resume falls
through to Verkstead's, rather than a write that guesses.

## Acceptance criteria

- [ ] The session id the store names is written onto the record as the records
      reader finds the row, and is there for a device that never ran the
      session.
- [ ] An opencode session transferred mid-turn resumes into the same session on
      the receiving device.
- [ ] The row's working directory in the carried database is this device's
      Worktree path, and the resume's own validation passes against it.
- [ ] A database this build cannot read, or a row it cannot find, is left
      untouched and falls through to Verkstead's Resume.
