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

- [x] A real OpenCode session's records reach the Transcript while it is still
      running, matched to that session by the Worktree it opened in and the
      moment it started — not to an earlier session in the same Worktree, and
      not to a session of another Conversation.
- [x] Each record is stored whole and verbatim, carrying its own kind and its
      place in the session's sequence, and a second poll takes only what arrived
      since the first.
- [x] A store that cannot be read — missing, locked out, or a shape this build
      does not know — leaves the session Capture-only and says so in the log,
      with the session itself unaffected.

## What the store turned out to be

**opencode 1.18.25 again** — the release tasks 01 to 03 pinned — pulled down as
`opencode-linux-x64` and driven outside Verkstead, because there is still no
`opencode` on the system profile and no provider account: the binary unpacked
into a scratch HOME whose XDG directories resolve inside it, on the line
`Agents::argv` writes (`-m provider/model`, `--prompt`, `--auto`), against a
stand-in OpenAI-compatible provider that says a little, calls the shell tool
once and then says what came back.

The store is `<the Profile's home>/.local/share/opencode/opencode.db`, the name
`OPENCODE_DB` pins, in write-ahead-log mode. Two of its tables are the whole of
what Verkstead reads:

- **`session`** — one row per session, with `directory` (the directory opencode
  was launched in, which for a Verkstead session is the Worktree), `parent_id`
  (null for a session opencode did not start under another), and `time_created`
  in milliseconds. Discovery is one statement over those three.
- **`event`** — one row per record, keyed by `aggregate_id` (the session) and
  `seq`, numbered **from zero** within the session, with `type` for the kind and
  `data` for the payload as JSON text. `session.created.1`,
  `session.updated.1`, `message.updated.1` and `message.part.updated.1` are what
  a plain turn writes; a streaming part is re-emitted under one id as it grows,
  which is task 05's to fold rather than this task's.

`session_message` is there beside them and was empty through every run — the
records are in `event`. A short two-turn conversation left 30 of them.

**What was proved against a live one.** With one session already in the store
from an earlier run of the same Worktree, a second was launched and the reader
pointed at the database while opencode was still writing it: the first poll took
the *new* session's records from sequence zero, the polls after it took nothing
while the model was thinking, and the poll after that took the fourteen that had
arrived since — cursor moving 8 → 9 → 23, read-only, with opencode holding the
file the whole time. The earlier session in the same Worktree was never read.

**And the Capture-only rule is a shape rather than a guess.** SQLite answers a
statement no schema can satisfy with its generic error code, and every other
code is a condition of the moment — the file not there, the writer's lock, a
half-made database. So the reader gives up on the first and looks again on the
second, and the suite proves the give-up by renaming the column the directory is
recorded under out from under a running session.

## What is still waiting

The same half tasks 01 to 03 left waiting, and for the same reason: **a session
Verkstead launched under an OpenCode Profile**, whose store the running server
follows. A session's `PATH` inside the sandbox is fixed and `opencode` is not on
it, so the end-to-end test writes the store from outside the sandbox — there is
no `sqlite3` on the system profile for a stub to write one with either — and
what that leaves unproved is only that a real opencode puts its store where
Verkstead binds it, which task 01 measured directly. The reading, the
discovery, the cursor and the Capture-only fallback are all proved against a
real store here.
