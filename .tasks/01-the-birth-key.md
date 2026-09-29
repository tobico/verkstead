# 01. The birth key, and one row per Conversation

## What to build

Every Conversation gains a cluster-wide **birth key**: the Device Id of the
machine it was drafted on, and the id it was given there. It is stamped at
creation and never changes afterwards, so a Conversation that has moved twice
still answers to the key it was born under. Every existing Conversation is
stamped too, with this device's own id and its own local id — which is exactly
what a Conversation that has never moved has.

Beside it, a mark saying a copy has been transferred and which device has the
live record. A Conversation carrying that mark is a tombstone: it holds the id
so that old links keep working and so that a transfer back has somewhere to
land, and nothing writes to it.

Two things read the pair. The **Merged List** keeps one row per birth key —
the live copy — and drops the tombstones, so the sidebar draws the work once
however many machines hold a copy of it. And a URL naming a copy that is not
the live one redirects to the device and id that is, the way the Terminal
pane's old path redirects to Code. The redirect is unconditional: a transferred
copy is never drawn, whether or not the device holding the live record is
answering.

`conversations` is STRICT and there is no migration machinery below it, so the
birth key and the mark arrive the way every other fact about a Conversation
has — a column added through `ALTER TABLE` where the value is one string, or a
table beside it. Follow whichever of the two the neighbouring facts already
use.

Nothing moves yet. What proves this task is two copies of one birth key across
two devices, written to the two stores directly, and the sidebar and the URL
behaving as though a transfer had put them there.

## Acceptance criteria

- [ ] Every Conversation carries a birth key — ones created after this lands,
      and every one that existed before it, the older ones naming this device
      and their own local id.
- [ ] Two copies of one birth key across two linked devices, one of them marked
      transferred, draw exactly one row in the sidebar: the live one.
- [ ] Opening the transferred copy's URL lands on the live copy's device and
      id, and does so whether or not that device is reachable.
- [ ] A lone Verkstead with no cluster draws its list exactly as it did before,
      the birth key saying nothing on a machine with nobody to say it to.
