# 06. A set-level file

## What to build

The comment box's plus-menu gains its *Attach file…* row, completing the whole
menu ADR-0028 gives it. A file put there is a **set-level attachment**: keyed by
the Set alone, under no question's label, stored and served through the same
attachment path an Answer's file uses, drawn as a pill under the comment box,
removable with × while the Set waits and refused once it is settled.

It is not an Answer to anything. A Set with a set-level file and no Answers is
as unanswered as it was, and the Submit treats it so. The agent's prompt lists
it under the Set's title with no question, the digest names it, and the
Response carries it as a set-level list beside the Answers rather than inside
any Answer's `attachments`. The settled Set page draws it under the comment.

Read the CONTEXT.md entries for Attachment and Question Set, and `docs/copy.md`
for the row's words.

## Acceptance criteria

- [ ] The comment box's menu offers *Attach file…*; a file put there is stored keyed by the Set alone, drawn under the comment box, removable while the Set waits, and refused once settled.
- [ ] A set-level file alone leaves every question unanswered.
- [ ] The prompt lists it under the Set's title with no question, the digest names it, and the Response carries it as a set-level list that `verkstead ask` and `verkstead answers` print.
- [ ] The settled Set page draws it under the comment, and a file on an Answer is drawn as before.
