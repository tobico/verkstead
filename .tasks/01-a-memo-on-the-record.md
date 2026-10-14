# 01. A memo on the record

## What to build

The `memo` Event kind, end to end from the store to the card on the Timeline,
with nothing yet able to leave one but a test. Read the **Memo** entry in
`CONTEXT.md` and ADR-0023 first: they are the agreement this backlog builds.

A memo is an Event whose body is the markdown the session wrote, in the `body`
column like the Brief and the Handoff. Beside it, a side table of the memo's
own — joined in the way a Steer's record is — holds what the body does not:

- `left_by` — the words the card and the prompt stamp it with, computed at the
  moment it is left and kept as text: *the session that worked task 03*, *the
  finish session*, *the session that planned the stage*, *the inline session*,
  *the instructed session*. Stored as words rather than as a step, because the
  record says what ran and a step is a reading of the runner's;
- `left_in` — the Event id of the session's own output, so the card can say
  which session that was;
- `retired_at`, `retired_by` and `reason` — empty until task 04 fills them, but
  declared now so the table is right from the first row. `retired_by` is words
  on the same pattern, or *you* for the human.

The side table is one a Conversation's rows live in, so the Transfer's carried
manifest names it (its `event_id` and `conversation_id` renumbered the way the
steers table's are), and the delete's conversation-keyed list empties it. Both
are held against the schema by tests that will fail until they do.

The viewer gets a `MemoEvent` beside the `HandoffEvent`: id, when, the body as
sanitized HTML, the `left_by` words, and an optional retirement (by whom, why,
when). The card is the Handoff's shape — a head reading **Memo 42** with the
stamp beside it, the body clamped to a few lines, the whole memo in the details
pane when opened — and a retired one is drawn as retired, dimmed, with the
stamp of who retired it and the reason under the head. The retired state has no
way to be set yet; draw it from the fixture.

A memo boards no Share: it joins the Handoff and the session's output on the
side of the filter that stays home.

## Acceptance criteria

- [ ] A memo written straight into the store reads back off the Timeline as its
      own kind with its body, its stamp and no retirement, and the viewer draws
      it as a card that opens into the whole memo.
- [ ] A retired memo in the fixture draws as retired, with who and why.
- [ ] The every-table-carried test and the delete-list test both pass with the
      side table declared, and a Transfer carries a memo with its stamp.
- [ ] A Share built from a Timeline holding a memo leaves it out.
- [ ] The viewer's generated types and the web suite's fixtures are
      regenerated and committed with the change.
