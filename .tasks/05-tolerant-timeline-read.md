# 05. An unknown Event kind no longer breaks a Timeline

## What to build

The store's Timeline read refuses an Event of a kind it does not know, and
takes the whole Conversation's Timeline with it. A memo is the first Event kind
added since cluster mode shipped, so a record carried to a device on an older
build now lands there with a kind that build refuses — and the next kind after
this one will do the same to a device on this build. Make the read tolerant,
here, so that from this build on an unknown kind is drawn rather than fatal.

An Event of an unknown kind reads as a **Notice** whose body says what it is:
that this Conversation holds an Event of a kind this build does not know,
naming the kind, and that a newer build will draw it. A Notice because that is
what it is — Verkstead saying something on its own account, after the fact,
with nothing to do about it — and because a Notice already has a card, boards
no Share, and is read by every build there is. The rest of the Timeline reads
exactly as before, and nothing is written: the row keeps its kind for the
build that knows it.

Say in the read's own comment why this is here, and leave the Memo kind's own
read in place beside it — the tolerance is for the kind after this one.

## Acceptance criteria

- [ ] A row of kind `whatever` inserted by hand into a Conversation's Timeline
      reads back as a Notice naming the kind, and the viewer draws it.
- [ ] Every other Event on that Timeline reads as before, in order.
- [ ] A Share of that Timeline is built without error and leaves the Notice
      out, as it does every Notice.
