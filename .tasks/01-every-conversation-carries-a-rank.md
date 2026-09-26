# 01. Every Conversation carries a rank

## What to build

A **Rank** on every Conversation: a fractional-indexing key with the device
that issued it suffixed after it. Nothing about what the sidebar draws changes
in this task — the order-by, the whole-list order route and the drag are all
left exactly as they are. What lands is the column, filled for every row there
is and for every row started from now on.

**The key arithmetic is written here rather than taken as a dependency.** It is
the published fractional-indexing scheme and not a variant of it: base62 over
`0-9A-Za-z`, with the leading character encoding the length of the integer part,
which is what makes ranking above the current top unbounded and compact —
`a0`, then `Zz`, `Zy`, … `Z0`, then `Yzz`. That matters because ranking above
everything is the hot path here: it happens at every start. One function does
the whole of it — *a key strictly between these two, either of which may be
absent* — where nothing on the left means **above everything** and nothing on
the right means **below everything**. A property test over a few thousand
random inserts, each asserted strictly between its neighbours and the whole
list asserted still in order, is what makes writing it rather than taking it
safe.

**A rank is the key, a separator, and this device's id.** The separator has to
sort *below* every character of the alphabet — `-` does, and `0` is the lowest
character the alphabet has — for two reasons: ordering the suffixed strings is
then ordering by key and then by device, and a key minted between two
neighbours still sorts between them once the suffixes are on. The arithmetic
never sees a suffix: it comes off before the key is computed and the new one
goes back on after.

**The rank is a `rank TEXT` column on the Conversation row**, not a table beside
it. The ADR says a device keeps a rank string on its own Conversations; stage 09
carries the rank on the copy it makes of a transferred one. The Conversations
table is STRICT and the column arrives the way the eight column-arrival rewrites
in the migrations module arrive: nullable in the schema, filled by a one-time
rewrite, and never written null by anything after.

**The rewrite ranks every existing row in the order the sidebar shows it in
today**: the placed ones in their place order, the unplaced above them newest
first, which is exactly what today's order-by says. It runs once, guarded by
the presence of what it rewrites rather than by a version number, like every
other rewrite in that module.

**How this device's id reaches the minting.** The database is opened *before*
the device identity is read, because the identity reads the membership out of
that same pool — so a rewrite running inside the open cannot know the id. The id
is passed in instead: the start paths take it, the rewrite takes it, and the
serve path runs the rewrite right after the identity is issued and before any
route is answered.

**Minting above the top reads the top and writes the row in one transaction.**
Two Conversations started a moment apart would otherwise mint the same key, and
they carry the same suffix, so the suffix does not save them. Every start path
funnels through one place already, which is where this goes.

## Acceptance criteria

- [ ] A database written before this change opens with a rank on every
      Conversation, and read in rank order they are in the order the sidebar
      shows today — placed ones by their place, unplaced above them newest
      first.
- [ ] Two Conversations started back to back hold different ranks and sort
      newest first, and so do a hundred started in a row.
- [ ] Two devices ranking above their own empty list mint ranks that sort
      apart, and a key minted between two neighbours sorts between them with
      every suffix on.
- [ ] The sidebar still draws, drags and saves exactly as it did.
