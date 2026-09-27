# 04. The cross-device drag

## What to build

A card dragged anywhere on the merged list lands where it was dropped, and the
order reads the same from every device afterwards. One **Rank** is computed
between the merged neighbours whichever devices those belong to, and it is
written to the device that owns the moved row and to nobody else.

**The viewer names the neighbour by device and id**, because an id alone is not
a row on a merged list. The hub has every rank in hand — its own in its store
and each member's in the list it holds — so it mints the key itself and never
asks a member what its neighbours are:

    PUT  /api/ui/conversations/rank
    { "row":   { "device": <id>|null, "id": <n> },
      "below": { "device": <id>|null, "id": <n> } | null }

`below` is the row it now sits directly under and `null` is the top of the list,
which is the sentence the drag and the arrow keys already send. The rank that is
minted carries the suffix of the device that owns **the moved row**, whoever its
neighbours belong to — that is what makes it that device's rank to keep.

**What the owning device is told is the rank itself.** Stage 05's route says a
move as *this row, under that one*, by an id that device's own database
numbered, so it cannot carry a drag whose neighbour is a row it has never heard
of. The route a hub calls says the other sentence — *this row's rank is now
this* — and it is reached on a member through the Relay at
`/api/ui/members/{device}/…` like every other call. The hub calls it on itself
for its own rows too, so there is one way a rank is written and the local case
is not a second one.

**The mint is serialised on the hub.** Stage 05 read both neighbours and wrote
inside one transaction so that two drops into one gap a moment apart could not
mint the same key with the same suffix. Minting off a held merge gives that up
unless the hub holds the two apart itself, so it must: two drops in a row are
the ordinary case, and two rows at one key is precisely the state this list
cannot represent.

**Two rows at one key is the exception, and this is what it does.** It is the
first rows of two devices rather than a rare case — two devices each ranking
above their own top mint the same key — and it sits at the top of the list,
where cards are dropped most. The two sort apart, which is what the merge needs
of them, but no rank sits between them: every rank at that key reads
`key-<device>`, and the row being moved carries its own. So `ranks::between`
refuses, and the hub **opens the gap first**:

1. Re-rank the *lower* of the pair through its own device — a key strictly
   between the pair's shared key and whatever is under that row, suffixed with
   that row's own device, written by the route above.
2. Then mint the dropped row's rank into the gap that opened, and write it to
   its own owner.

Two devices are written to in that one case and one in every other, and the drop
lands where the human put it. It does not recur at that spot: the pair no longer
share a key. Where the lower row's own device cannot be reached the gap cannot
be opened, so the drag is refused and the sidebar says so — naming the device
that could not be reached, in the error line it already draws under the list
when an order could not be saved.

**A neighbour that has gone since the list was drawn is still not a refusal**,
and a rank written to a member is read back on that member's next Nudge, which
is what carries the new order to every other open workbench.

CONTEXT.md's **Rank** entry gains what a drag into that one gap does, which it
currently leaves to this stage.

## Acceptance criteria

- [ ] A member's row dragged between two of this device's rows, and one of this
      device's dragged between two of a member's, both hold after reloading
      either device — and after restarting either.
- [ ] The only device written to is the one that owns the moved row, except when
      the gap has to be opened, where the lower row's own device is written too.
- [ ] A row dropped into the gap between two rows the devices minted at one key
      lands where it was dropped, and a second drop into that same gap needs no
      gap opened.
- [ ] Two drops into one gap in quick succession mint two different ranks.
- [ ] CONTEXT.md says what a drag into that gap does.
