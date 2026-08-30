# 04. The session store read and followed

## What to build

Finding the session opencode kept of itself, and putting what it wrote there on
the Timeline as it is written.

**The store is a SQLite database, not a file of lines.** That is the one thing
about this stage that moved since it was planned. opencode keeps one database per
account under its data directory — the file whose name task 01 pinned — in
write-ahead-log mode, with a row per session and a row per record within it. The
brief expected JSON files appended to; they are gone.

So the follower Verkstead has does not fit. It remembers a path and a byte
offset, reads what has been appended since, and splits whole lines off the end —
none of which a database has. What it does have is a **sequence number per
record within a session**, which is the same idea in the store's own terms: the
cursor is the highest sequence already taken, and each poll takes what has
arrived past it.

**ADR-0006 is unchanged and this is why it survives.** Every record's payload is
JSON, so a record still reaches the Transcript verbatim and is parsed at render
time, exactly as a Claude line or a Codex rollout line is. Store the record whole
— its own kind and its sequence alongside the payload, so the renderer has what
it needs and nothing is invented on the way in. Nothing here parses one, and the
two things a Timeline row is summarised by are still read by the crate with the
parser in it.

**Discovery is by the Worktree and the moment, as Codex's is.** opencode takes no
usable session id at launch, so nothing Verkstead knows beforehand names its
session. What identifies it is what the session wrote about itself: the row whose
recorded directory is this Conversation's Worktree — bound into the sandbox at
the path it has outside one, so the two are the same string — and which was
created at or after the moment this session was launched. Take the newest such
row where more than one matches, the way the rollout finder does, and allow the
same slack for a coarse clock. The rule is Codex's; only the mechanism differs.

**Read it as an outsider.** The database belongs to a program that is writing it
while this reads, so open it read-only and expect to be locked out
momentarily rather than treating that as a failure. A poll that cannot read is a
poll that looks again, on the cadence the byte relay already flushes on.

**A store this build cannot read leaves the session Capture-only.** The layout is
explicitly the backend's own and moves between releases: a table renamed, a
column gone, a database this build does not understand. None of that may fail a
session. The Capture is a complete record on its own, and a session with no
Transcript has always been an ordinary thing here — the same answer a stub agent
and a log that never appeared both get. Make that the failure mode deliberately,
and make it visible in the log rather than silent.

Nothing here draws anything: the records land as lines the pane cannot yet read,
which is what task 05 is for. The proof of this task is that they land at all,
while the session still runs, and that the row's turn count moves.

## Acceptance criteria

- [ ] A real OpenCode session's records reach the Transcript while it is still
      running, matched to that session by the Worktree it opened in and the
      moment it started — not to an earlier session in the same Worktree, and
      not to a session of another Conversation.
- [ ] Each record is stored whole and verbatim, carrying its own kind and its
      place in the session's sequence, and a second poll takes only what arrived
      since the first.
- [ ] A store that cannot be read — missing, locked out, or a shape this build
      does not know — leaves the session Capture-only and says so in the log,
      with the session itself unaffected.
