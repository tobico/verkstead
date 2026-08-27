# 04. Header state labels

## What to build

The conversation header shows the branch name and nothing about state — Done is
visible only as a dimmed sidebar card and a Moved row at the bottom of the
record. The ended states get quiet header labels so the answer is one glance
away.

**Done** and **Closed** conversations show their state word in the header, in
the same quiet style task 01 gave the **Stopped** label. Active states keep the
clean header — no state word for Grilling, Implementing, Wrapping or Follow-up;
the existing *Waiting on checks* label and the Stopped label already carry the
conditions worth a word. Done and Closed labels are plain words, nothing to
press.

## Acceptance criteria

- [ ] A Done conversation's header says **Done**; a Closed one says **Closed**,
      in the Stopped label's style
- [ ] Active states show no state word in the header
- [ ] The existing header labels and controls sit unchanged beside the new word
