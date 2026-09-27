# 02. The row, the header and the spoken row

## What to build

The merged list says which machine each piece of work is on, in the three places
a Conversation is named: the row, the pane header it opens, and what a screen
reader reads.

**The row's second line reads the OS icon, the device, then the repo**, where
the branch is the first line. On every row once anything is linked, **this
device's own rows included** — that is what makes it one list rather than this
device's list with visitors on it. Where nothing is linked the line is the repo
alone and there is no icon, which is every Verkstead that is not in a cluster
and so is the shape nearly every row is drawn in today.

The icon is the mark a device already wears wherever it is drawn — the Devices
section of the Remote access pane and the modal asking whether to let a device
in — so a WSL draws the Linux mark and a word this build has no mark for still
draws a row. It is read off the OS word the row carries rather than matched
anywhere new.

**The pane header carries the device beside the branch**, where the repo already
stands. Drawn under the same rule as the row: the device wherever there is a
cluster to name one, and nothing at all where there is not. The Conversation
pane knows which device it is on from the URL it is drawn at; what it needs
beside that is the name and the mark, which is the Devices reading — that has
this device's own identity and every member's in one shape, and it moves on the
`devices` Nudge.

**The row read aloud says the device**, in the sentence that already carries the
branch, the repo, the state and what the mark would have said. The card says
where the work is in marks rather than in words and a mark is nothing to a
screen reader, so the device belongs in that label for the reason everything
else in it does.

**An unreachable member's rows are dimmed and say so.** The row keeps
everything it had — it is the last list the hub held of that device — and reads
*unreachable* where it is read aloud, beside the device it names. The dimming is
the sidebar's own existing treatment for work that has stopped; what is new is
that a row can wear it for a reason that is about the machine rather than about
the Conversation. A press on such a row is refused by the Relay, by name, which
is stage 04's doing and nothing to add here.

## Acceptance criteria

- [ ] With a member linked, every row's second line reads the OS icon, the
      device name and then the repo — this device's own rows the same as a
      member's.
- [ ] With nothing linked, no row draws a device or an icon, and the pane header
      draws none either.
- [ ] The pane header of a member's Conversation carries that device beside the
      branch, and the row read aloud names it.
- [ ] An unreachable member's rows are drawn dimmed and say *unreachable* when
      read aloud.
