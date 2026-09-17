# 04. `.claude.json` copied, pre-seeded and merged back

## What to build

`.claude.json` is **copied** into the session's profile rather than linked, on
all three platforms. The pre-seeding is written into the copy, not into the
account's file.

**The copy.** Before seeding, strip `mcpServers` from it: the top-level key and
the key under each `projects` entry. Those are the human's own MCP servers, the
same leak as plugins. Then pre-seed a `projects` entry, with
`hasTrustDialogAccepted: true`, for:

- the Repo's main checkout path, which Claude checks first;
- the Worktree path, which Claude checks next as it walks up from the launch
  directory.

Use plain paths, with forward slashes on Windows, as Claude keys them.
**Do not seed `bypassPermissionsModeAccepted`.** Claude Code 2.1.268 migrates
that key out of `.claude.json` into `settings.json`, and task 03's settings key
already covers the consent.

On Linux the copy is bound over `$HOME/.claude.json`. Claude saves the file by
temp file and rename, falls back to writing in place when the rename is busy,
and so writes the copy under `homes/<id>`. On a Mac and on Windows it is a file
in the profile itself.

**The write-back is a merge, not a copy, and it runs on all three platforms.**
A copy never shares an identity with the account's file, so the identity check
alone would overwrite the account at every session end. Sessions run in
parallel, so the last to end would win over what other sessions, or the human's
own `claude`, wrote in the meantime. And a whole-file write-back of a stripped
copy would delete the human's MCP servers. So, at session end:

1. Keep the copy exactly as it was given to the session (after stripping and
   seeding) as the baseline.
2. Read the session's copy as it is now, and the account's file as it is now.
3. Into the account's file, write only the top-level keys, and the individual
   `projects` entries, whose value in the session's copy differs from the
   baseline. Keys the session removed are removed. Keys stripped from the copy
   (`mcpServers`) are never written or removed. The seeded trust entries reach
   the account only where the session changed them. This is a narrowing of
   ADR-0014's whole-file write-back, and the human approved it.
4. Leave the account's file untouched when nothing differs. Write it by temp
   file and rename, keeping its mode.

The identity-based write-back for linked files (the credentials file on a Mac
and Windows) stays as it is. Today `written_back` re-links after it copies,
which is wrong for a copied file, so `Closing` needs to tell a linked file from
a copied one. Linux and a Mac now hand back a non-empty `Closing` too. A file
that cannot be merged back is logged and the rest go on; nothing is refused.

## Acceptance criteria

- [ ] Inside a session, on all three platforms' suites, the Repo and the
      Worktree read as trusted in `.claude.json`, and no `mcpServers` is
      present.
- [ ] A key changed inside reaches the account at session end on all three,
      while a key the account changed during the session is kept, and the
      account's `mcpServers` survive.
- [ ] A session that changed nothing leaves the account's `.claude.json` byte
      for byte as it was.
- [ ] A linked credentials file still one with the account's is left alone.
