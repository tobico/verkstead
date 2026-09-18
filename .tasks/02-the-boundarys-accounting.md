# 02. The boundary's accounting

## What to build

A first Windows boundary took 112 s to write. The log already says a boundary
was slow and already names the entries that dawdled past a quarter-second
threshold — but sixty entries each under that threshold cannot make 112 s, so
the time went somewhere no line describes. This task makes the log add up.

Three things are wrong with the accounting as it stands, and all three are
readable in the source rather than needing a machine to find:

- **The total spans work no per-entry line covers.** The clock starts, and then
  an access-control list is read for every step entry — deciding whether the
  account can already walk through that directory — before the loop that times
  anything begins. Those reads are inside the total and outside every line
  beside it.
- **Whole stretches of writing a boundary are outside the total altogether.**
  Before a word of it is written, the description's paths are read to learn
  which were inheriting; the session account is resolved; the Conversation's
  entries are opened and the record of them written down; the machine's own
  record of the standing entries is written; and before all of that the
  rendering empties the profile and makes the junctions the entries go on. None
  of it is timed and any of it could be the 112 s.
- **The total names a count it did not walk.** It reports the whole
  description's number of entries while the loop walked the shorter list of the
  ones worth writing, so the count in the total already disagrees with the
  count of lines beside it.

What lands: every entry's duration at debug — not only the ones over the
threshold, which is what makes a sum possible at all — and a total that either
spans the same work the per-entry lines describe or names the stretches it
covers that they do not. The count the total reports is the count of entries the
lines beside it describe. The stretches listed above each get a line naming what
they are and how long they took, so a slow start says *where* rather than only
*that*.

**No behaviour changes.** Nothing about what a session may reach moves; this is
the log saying what it already does honestly. The threshold that decides which
entries are worth an `info` line stays as it is — the debug lines are beside it,
not in place of it.

**Confirming the 112 s on a real Windows machine is outside this task.** The
fix is what the source shows; whether the numbers then account for the reported
112 s is a measurement the human takes separately. Access-control writes are not
faithful under wine, so a run here proves the lines are emitted and sum
correctly, not what they sum to on the reporter's machine.

## Acceptance criteria

- [ ] Every access-control entry written for a boundary has its duration logged
      at debug, and the sum of those durations plus the named untimed stretches
      accounts for the total the log reports.
- [ ] The count the total names is the count of entries the per-entry lines
      describe.
- [ ] The reads that decide whether an entry already stands, the inheritance
      read, the account resolution, the record writes and the rendering are each
      timed and named in the log, so a slow boundary says which of them took the
      time.
- [ ] Nothing about what a session may reach changes, and the existing tests on
      the entries a description produces still pass.
