# 02. An unreadable Set costs its own row and nothing more

## What to build

A stored Question Set the server can no longer deserialize is drawn as
unreadable in place, and takes nothing else down with it.

Four of this instance's fifteen Conversations cannot be opened at all: reading
one answers *the Conversation could not be read*, because a single Set on its
Timeline fails to parse. The cause is ordinary schema movement — the stored
bodies carry `accepted_by` on a `proposal`, a field since retired, and the
schema denies unknown fields — and it will happen again every time a field
leaves. The database holds no migration machinery by design, so the fix is not a
migration: it is that one unreadable record must not be able to cost a whole
Timeline, a whole Conversation and every Event beside it.

This is the rule ADR-0006 already takes for Transcript lines, applied to stored
Sets: keep what was written, and defer rendering rather than lose the record.
A Set that will not parse becomes a row that says so, with its stored body
reachable for anyone who wants to read it, and the Timeline around it draws
normally. The same holds on the Set's own page.

Nothing rewrites or deletes the stored bodies. They are the record of what was
asked, and a Verkstead that could read them again later should still find them
there.

## Acceptance criteria

- [ ] The four Conversations that answer *the Conversation could not be read*
      today open, with every readable Event on their Timelines drawn as usual.
- [ ] An unreadable Set is a row saying it cannot be read, with its stored body
      reachable, rather than an omission or a failed page.
- [ ] Asking for such a Set on its own page gives the same account of itself
      instead of failing, and answering or archiving it is not offered.
