# 13. Timeline rows put straight

## What to build

Two presentation changes on timeline items:

1. **Question-set rows.** In the question-set item's interview list, the
   question text goes bold and both the question and the answer truncate to
   one line each with an ellipsis. The rows are grid items holding spans, so
   truncation needs the usual care (block-level text, min-width zero). The
   item's outer card — border, padding, selected and waiting accents — is
   untouched: it was settled that only inner borders were ever to go, and on
   this branch the interview layout already has none.
2. **Task and stage numbers.** On the pinned task-list and stage-list items,
   the number moves from before the text to the right edge of the row —
   `[ ] Some task            01` — box and text leading, number flush right
   in its monospace style, done-strikethrough unchanged.

## Acceptance criteria

- [ ] Long questions and answers each hold to one truncated line; labels,
      nesting and the unanswered marks render as before
- [ ] Task and stage numbers sit flush right on every row, phone widths
      included
- [ ] The question-set card's border, padding and selected/waiting styling
      are unchanged
