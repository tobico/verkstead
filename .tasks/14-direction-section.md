# 14. The Direction section squared away

## What to build

Two changes on the set pages (chooser and record alike):

1. **Direction becomes a section like the others.** The recent restyle
   dropped its "Direction" heading and left the content bare. Restore the
   house pattern — heading outside, card inside: a "Direction" section
   heading like Preface and Postscript have, with the whole of the content
   (the rationale and the chooser or the recorded pick) in a card styled
   like a question's, keeping the "End" label in the label position the way
   a question keeps its Q-number. Include the card in the wide-window gutter
   rules the other cards share, which also fixes the End label currently
   hanging outside the column there.
2. **One nothing-answered notice goes.** The line "Nothing here was
   answered, and nothing was said about the Set either: every question went
   back to the agent still open." — shown on a set answered with no answers
   and no comment — is removed. Its two siblings stay: the counter-question
   variant (answers empty but a comment present) and the archived-unanswered
   line.

## Acceptance criteria

- [ ] Direction renders as a headed section holding one question-like card,
      on the chooser and on the settled record
- [ ] The End label aligns like a question label at every width, gutter
      layout included
- [ ] The quoted notice never renders; the other two variants still do
