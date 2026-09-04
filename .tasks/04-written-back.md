# 04. A replaced link is written back

## What to build

**A hard link is one file only while everything writes in place.** An agent that
saves its config by writing a temporary file and renaming it over the top leaves
the session writing to a file of its own, with the account's copy seeing none of
it and nothing saying so — and an agent's config is exactly that kind of file.

What is decided is the outcome rather than the mechanism: **nothing a session
wrote to its account is lost.** As the session ends, a linked file that is no
longer the account's own is written back over it, and the link is made fresh for
the session after. The ordinary case is one file and costs nothing; the
replacing case costs a copy rather than the session's work. Directories are not
in it — a junction is a path rather than a file, and nothing replaces one.

**The seam does not exist yet, and this task is what builds it.** The renderer
makes the links as it renders and nothing runs at the far end of a session at
all: a session ends in the relay and a terminal ends in its own follow loop, and
by then neither has anything of the Sandbox left. So the Sandbox hands back a
**closing value** alongside the command it rendered — held for as long as the
thing it started runs, and asked to close when that thing has gone. Both callers
hold one: a session's relay and a Conversation Terminal's follow loop, since a
terminal runs in the same profile under the same account.

The closing value is nothing on the two Unix platforms, and on Windows it is
what knows which files were linked where. Whether a linked file is still the
account's own is asked of the file rather than remembered — two names for one
file is a fact the filesystem holds.

Write-back failures are logged rather than raised: a session that has ended is
past refusing, and what one costs is worth naming in the log with the file it
was about.

## Acceptance criteria

- [ ] A file linked in from the account, replaced inside the session by writing
      a temporary file and renaming it over the top, is on the account once the
      session has ended, holding what the session wrote.
- [ ] The session after it finds one file again rather than two: the link is
      made fresh, and a change written in place inside is visible on the account
      without anything being copied.
- [ ] A file the session only wrote in place is not copied back — the account
      already has it, and nothing is rewritten.
- [ ] A Conversation Terminal closes the same way a session does, and neither
      Unix platform grew anything to close.
