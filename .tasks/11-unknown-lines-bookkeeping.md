# 11. Unknown lines to bookkeeping

## What to build

Transcript lines whose top-level type the reader does not know stop landing in
the main record as "a line this version does not know". They fold into the
bookkeeping group at the end of the record instead, under their own kind —
`atis-latch`, the type that surfaced this, along with any other unknown
top-level type. The bookkeeping list stays a closed list plus this one
catch-all for unrecognised top-level kinds.

The boundary is the settled decision:

- **Unknown top-level line types** go to bookkeeping — visible in the fold,
  out of the conversation.
- **Unknown blocks inside** an assistant or user turn stay inline as the
  unread row — they are part of the conversation, and a silent miss there
  would hide real content.
- **Lines that are not JSON at all** stay inline too, for the same reason.

The transcript module's docs and the ADR about reading the session log both
state the old shown-inline rule for unknown kinds; both get a note saying what
changed and why. Unread turns counted toward the turn count; folded lines do
not, matching how bookkeeping is counted today.

## Acceptance criteria

- [ ] A line with an unknown top-level type renders in the bookkeeping group
      under its own kind, not in the main record
- [ ] An unknown block inside a known turn, and a non-JSON line, still render
      inline as unread rows
- [ ] The existing transcript tests for unknown kinds are updated to pin the
      new boundary, and the module docs and ADR carry the change
