# 06. Sidebar ordering and attention markers

## What to build

The conversations sidebar is ordered by hand and remembers the order, and a
Conversation that wants attention carries a marker icon and a border rather than
the single dot it carries today.

The list is newest-first now, which the store's own comment calls a stand-in
until there is a manual order. Manual ordering is the design's decision: this is
one human's working set, and which piece of work sits at the top is theirs to
say. Dragging a row is how it is said. The order survives a reload, a restart of
the server and a second device; a Conversation that has never been placed lands
somewhere predictable and stated rather than wherever a sort happens to put it.

The order is Verkstead's own fact about its own list, so it is stored the way
every fact like it is stored here — beside the Conversations rather than as a
column on them, there being no migration machinery and that table being left
alone.

The marker is driven by what the sidebar already computes: whether something
about a Conversation is waiting on the human — an ask left open, blocking or
deferred, or an open Interruption. A Draft is not one of them and keeps being
drawn as a draft. What changes is the drawing: an icon and a border, legible at
a glance across a list, in place of a dot at the right edge — and it stays
legible on a phone, where most answering happens. Whatever a screen reader is
told now stays true, because the marks are the whole of what the row says about
where it has got to.

## Acceptance criteria

- [ ] Rows can be dragged into an order that survives a reload, a server restart
      and a second device, and a newly started Conversation lands in a stated
      place.
- [ ] A Conversation waiting on the human draws the marker icon and the border;
      one that is not, draws neither; a Draft is still drawn as a draft.
- [ ] Both the ordering and the markers work on a phone-width viewport, and the
      row still reads aloud as what it is.
