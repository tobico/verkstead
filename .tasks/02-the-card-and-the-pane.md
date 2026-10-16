# 02. The card and the pane

## What to build

The memo's card and its details pane, redrawn around the title task 01 put on
the wire. CONTEXT.md's **Memo** entry says what each shows; this is the whole
of it.

**The card** is the kind and the title: headed *Memo*, the way the Handoff's
and the Notice's cards are headed by their kind, with the memo's title as a
subtitle under the head, and nothing else — no stamp saying who left it, no
clamped body, no Retire button, and no number anywhere. A memo with no title
shows the first line of its body where the title would be, cut to one line
with an ellipsis as the title itself is. A retired memo's card strikes the
subtitle through and says nothing more; it is otherwise the same card. The
card still opens the pane, and is the pane's own card the selection lands on.

**The pane** is headed by the title — the body's first line for an untitled
one — over the whole memo, in the document pane the handoff opens in. Under
the document, an understated line in the quiet ink a stamp is set in: *Left by
the session that worked task 03 on 14 Oct 2026*, the date in the form the rest
of the workbench says one in, and the session's name a link that opens that
session's own output pane, in place, the way pressing its card would — the
first pane to open another Event, so the navigation goes through the same
path a card press takes rather than a second way of selecting. Where the memo
is retired, a second line beneath: *Retired by the session that worked task
04: moved back into the relay*, or *Retired by you*. And beside the line, the
one act on the memo: **Retire** on one that stands, **Restore** on one that
does not, one press with nothing asked first, the pane redrawing from what the
server answered the way the card does today. The error line for a press that
failed moves with it.

A share leaves memos out as it always has, so nothing here is drawn on one.

Follow docs/copy.md for every word on screen — buttons are the verb, sentence
case, no full stop — and CONTEXT.md for every name. Avoid *note*, *comment*
and *annotation*.

## Acceptance criteria

- [ ] The viewer suite shows a memo's card headed *Memo* with the title under
      it and nothing else — no stamp, body, button or number — the body's first
      line in the title's place on an untitled one, and a retired one's
      subtitle struck through.
- [ ] Opening a memo shows the title as the pane's heading, the whole memo,
      and the bottom line saying who left it and when, with the retirement on
      a second line where retired; pressing the session's name opens that
      session's output pane.
- [ ] Retire and Restore work from the pane in one press each, redrawing from
      the server's answer, and are nowhere on the card.
