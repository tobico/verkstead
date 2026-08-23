# 06. Question-set interview

## What to build

The question-set timeline card drops its table — it never fit the column —
and reads as an interview: alternating question and answer lines.

A question line always starts with its label (`Q1`, `Q7a`), styled exactly as
the detail page styles it — monospace, semibold, accent orange — with the
question text running beside it. The answer line beneath is indented far
enough to clear the label, so the question text and the answer text share one
left edge. Sub-questions are their own lines under their lettered labels,
rendered the same way.

**Every pair shows** — no clamp, no gradient; a long set earns a long card
(settled explicitly against clamping like the markdown cards). The existing
conventions for open sets carry over: a dash where an answer is still
awaited, the dimmed "unanswered" word where a closed set left one open. The
card's head (title, waiting badge) and its behaviour as a button opening the
full sheet in the details pane are unchanged; answers show the picked
option's text as the table showed it today.

## Acceptance criteria

- [ ] No table remains in the timeline card; each question renders as an
      orange-labelled line with its answer indented to the same left edge
- [ ] Sub-questions render under their lettered labels the same way, and
      every question in the set is shown
- [ ] Waiting and closed-unanswered answers keep today's dash and dimmed
      word, and the card still opens the sheet in the details pane
