# 01. The built root on Linux and a Mac

## What to build

A Claude session on Linux or a Mac no longer gets the account's whole
`~/.claude`. Instead, each session start empties and rebuilds
`homes/<conversation id>/.claude` under the Data Directory, holding exactly
this allowlist:

- **`.credentials.json`**, linked to the account's: a read-write bind over the
  name on Linux, a symlink on a Mac. Claude Code 2.1.268 saves it by temp file
  and rename, and falls back to writing in place when the rename fails as busy.
  So on Linux a bind still takes a token refresh, and no copy is needed. On a
  Mac the rename replaces the symlink, so the Mac rendering hands back a
  `Closing` naming the credentials file. The existing identity check then
  writes it back to the account. Where the account has no credentials file (a
  Mac keeps its login in the Keychain), the root gets none, and a file the
  session makes is written back.
- **Two `projects/` entries**, joined read-write (bind on Linux, symlink on a
  Mac), each made in the account first where missing:
  - the Repo's main checkout's entry, which holds Claude's per-Repo memory;
  - the Worktree's entry, where the session's transcript is written.
- **Nothing else.** No `plugins/`, `commands/`, `agents/`, `skills/`,
  `CLAUDE.md`, `history.jsonl` or `settings.json`, and none of the rest of
  `projects/`.

How Claude Code 2.1.268 names a `projects/` entry from a path: every character
outside `[a-zA-Z0-9]` becomes `-`, one for one. A name longer than 200
characters is cut to its first 200, then `-`, then a hash in base 36. The hash
is a Java-style 32-bit string hash of the original path (`h = (h << 5) - h +
charCode`, kept to 32 bits), made positive. Memory is keyed by the main
checkout, which Claude finds through the Worktree's `.git` file and
`commondir`. The transcript is keyed by the launch directory, which is the
Worktree. Use the plain path, never a `\\?\` spelling.

**Linux gains a `homes/<id>` directory.** Today `Homes::for_conversation` gives
Linux the server's own HOME path, made empty inside the namespace by a `--dir`.
Linux now gets a `homes/<id>` too, made and emptied per session like the other
two platforms. The rendering binds the built `.claude` over `$HOME/.claude`
inside the otherwise empty home. The rest of `$HOME` inside stays as it is
today.

**The `.claude/skills` cover goes.** It hid the account's own skills, and a
built root has none to hide. `/verkstead/skills` is unchanged.

**Unchanged in this task:** `.claude.json` stays joined as today (task 04
replaces it). Windows keeps the whole-account join until task 02, so build the
root on the two Unix platforms only. The description in `Sandbox::surface` is
still one description, and `account_inside` is still what the onboarding
wizard's `account_in_home` reads to detect an account, by `~/.claude` and
`~/.claude.json` both existing. Keep detection exactly as it is. What changes is
what a session is *given*. `DISABLE_AUTOUPDATER` stays set.

Session-log discovery reads `projects/` under the account's `claude_dir` on the
host, and walks one level of entries for `<session-id>.jsonl`. It keeps finding
the log through the joined Worktree entry. Stage 02 moves discovery into the
root; leave it alone here.

## Acceptance criteria

- [ ] In the Linux and Mac sandbox suites, the account's `settings.json`,
      `plugins/`, `CLAUDE.md` and another repository's `projects/` entry are
      not visible inside a session, and nothing is at `~/.claude/skills`.
- [ ] A file written inside under the Repo's memory entry and under the
      Worktree's entry lands in the account, and an entry missing from the
      account is made there. The Transcript still finds a session's log.
- [ ] A credentials file written inside appears in the account: in place on
      Linux, and replaced by rename on a Mac, written back at session end.
- [ ] The Linux suite sees the built directory under the Data Directory,
      emptied and rebuilt for each session.
- [ ] The `projects/` naming matches Claude Code's rule, with a unit test for a
      path longer than 200 characters.
- [ ] The Windows suite still passes unchanged.
