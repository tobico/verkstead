# 06. Lifetime

## What to build

When the files go, and when they do not.

**Close leaves them.** Closing a Conversation removes its Worktree and its
handoff directory and leaves its attachments directory alone: a Steer can bring
a Closed Conversation back to life, and a file cannot be made again the way a
worktree can. Trim leaves them too — the record has to read whole, and the
files are the human's own input rather than the bulk a session produced.

**Deleted takes them.** The Cleanup's delete, the one point Verkstead forgets a
Conversation for good, removes the attachments directory along with every row
the Conversation owned. The Cleanup touches disk for the first time here, so a
directory that will not go is logged and the rows go anyway, the way a
worktree git refuses to remove does not hold a close up.

**A sweep behind both.** At startup, every directory under the attachments root
that no Conversation in the record names is removed, in the shape of the
worktrees sweep and with its safety property: every candidate comes out of
reading that one directory, and nothing else is ever a candidate. It is the
backstop for a delete that failed halfway and for a database restored from
before an attachment was made.

The **Attachment** entry in `CONTEXT.md` says all of this; this task makes it
true.

## Acceptance criteria

- [ ] Closing a Conversation with attachments leaves its directory, and a Steer
      out of Closed runs a session whose prompt lists the files and whose
      sandbox reads them.
- [ ] The Cleanup's delete removes the directory, and a directory that cannot
      be removed is logged without stopping the delete.
- [ ] A stray directory under the attachments root is gone after a server
      start, and one a live Conversation names is not.
