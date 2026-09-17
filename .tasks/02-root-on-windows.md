# 02. The built root on Windows, with the grant narrowed

## What to build

A Windows session gets the same built `.claude` that task 01 gives Linux and a
Mac, under its existing profile `homes/<id>`, emptied and rebuilt per session:

- **`.credentials.json`**, joined by a hard link. Claude saves it by temp file
  and rename, which breaks the link. The Windows `Closing` already names
  hard-linked files and writes a replaced one back by file identity, so the
  credentials file joins that list. Where the account has none, the root gets
  none, and a file the session makes is written back.
- **The Repo's main checkout's and the Worktree's `projects/` entries**, joined
  by junction, each made in the account first where missing. Use the same naming
  function as task 01.
- **Nothing else.** The junction on the whole of `~/.claude` goes.

**The boundary grants the root, not the account.** Today one read-write
access-control entry covers the whole of `~/.claude` (1,289 files on the
reporter's machine, and the tree a first boundary walks), plus a refusing entry
over `.claude/skills`. Both go. Instead:

- one entry on the built root;
- one on each of the two joined `projects/` entries;
- **one on the credentials file itself.** A hard link shares the file's own
  access list rather than taking its new parent's, so without its own entry the
  session cannot read its login.

These entries are on the account's own paths, so they are recorded and taken
back with the rest of the Conversation's entries, the way the entry on
`~/.claude` is today, and swept at startup after a crash. `%APPDATA%`,
`%LOCALAPPDATA%`, `TEMP` and the rest of the fresh profile are untouched.

The one-volume rule (`across_volumes`) now applies to the credentials file
instead of the directory. `.claude.json` stays hard-linked as today until
task 04.

The Windows suites run here under Wine (mingw plus wine). ConPTY is the only
part that is not faithful.

## Acceptance criteria

- [ ] The Surface of a Windows Claude session names no entry on the account
      directory. The Windows suite's boundary is written over the root, the two
      `projects/` entries and the credentials file only.
- [ ] A session reads its credentials through the hard link, and a credentials
      file replaced by rename inside reaches the account at session end.
- [ ] The account's `plugins/`, `CLAUDE.md` and another repository's `projects/`
      entry are refused or absent inside.
- [ ] The credentials file's entry is gone once the Conversation is closed, and
      a crash's leftover entry on it is swept at the next startup.
