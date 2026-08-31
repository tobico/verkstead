# 03. Wrapping skips ignored comments everywhere it reads them

## What to build

Apply the ignore rules from task 02 wherever Wrapping reads a pull request's
comments: the fresh comments that would dispatch a batch session after the
review, and the pre-existing comments folded into the review prompt. Review
bodies already flow through the same comment stream, so they are covered by
the same check; companion repositories' pull requests go through the same
reading too.

The matching semantics are task 02's: a comment is ignored when any one rule
matches it, a rule matches when every field it gives matches (author regex
against the author's login, body regex against the markdown as written), and
patterns match anywhere in their text.

**A skipped comment is recorded as addressed at the moment it is skipped**, in
the same store the dispatch path already records through. That makes removing a
rule non-retroactive: months of a bot's nagging never floods back as sessions
when the human deletes the rule. The rules are read fresh from the settings on
every poll, so a newly added rule takes effect without a restart.

The ignore is about agent work only. Skipped comments still appear wherever the
workbench lists a pull request's comments — nothing about the details pane
changes.

## Acceptance criteria

- [ ] A comment matching a rule is never dispatched for, never reaches the review prompt, and is recorded as addressed when skipped — so deleting the rule afterwards resurrects nothing.
- [ ] A rule giving both fields ignores only comments matching both; rules combine with OR; matching works the same on conversation comments, review bodies, diff comments, and companion pull requests.
- [ ] A rule added while a wrap-up is already watching takes effect on the next poll, with no restart.
- [ ] The pull request details pane still lists skipped comments.
