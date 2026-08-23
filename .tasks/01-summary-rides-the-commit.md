# 01. Summary rides the commit onto the Timeline

## What to build

A commit's message body becomes its summary, and the pane shows it. The branch
sweep, which today keeps only the sha, the subject and the diffstat, also reads
the body of each commit it records, strips the trailing trailer block
(`Co-Authored-By` and kin — git keeps the full message regardless), and keeps
what remains beside the commit. A body that is empty, or that is only trailers,
counts as no summary at all.

The store keeps the body the house way: there is deliberately no migration
machinery, so it hangs off the commit's Timeline Event as a table of its own —
never a column added to the existing STRICT `commits` table — written in the
same transaction that records the commit, so exactly-once recording holds as it
stands.

The commit pane then renders the summary as markdown above the diff — the same
server-side rendering and sanitizing every other Timeline document gets —
between the commit's header and the diff, which is the order it is read in:
what the commit says about itself, then what it changed. A commit with no
summary draws the pane exactly as today. Commits recorded before this landed
stay as they are; there is no backfill.

## Acceptance criteria

- [ ] A newly swept commit with a message body shows that body, rendered as
      markdown, above the diff in its pane.
- [ ] A trailing trailer block is stripped from what is stored; a body that is
      only trailers, or empty, stores nothing and draws nothing.
- [ ] A commit without a summary, including every commit recorded before this
      task, draws its pane unchanged.
- [ ] A commit is still recorded exactly once per Conversation per sha,
      sweep overlaps and restarts included.
