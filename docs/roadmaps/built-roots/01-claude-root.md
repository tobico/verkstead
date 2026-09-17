# 01. Claude's built root

## Goal

A Claude Code session, on Linux, a Mac and Windows alike, runs in a `.claude`
that is Verkstead's own: the account's credentials are in it and a login from
inside writes through; the account's per-Repo memory is in it; a `settings.json`
Verkstead wrote is in it, so the session never stops at the bypass-permissions
consent; and nothing else of the human's is — no plugins, no hooks, no global
`CLAUDE.md`, no history, no other repository's transcripts. A fresh Windows
install with a plain npm `claude` presses Start work and the session works.

## Decisions in force

From [ADR-0011](../../adr/0011-agent-backends.md) and
[ADR-0014](../../adr/0014-windows-sessions.md), each under *Amended: the account
is built rather than joined*, and from the grilling that settled this roadmap.

- **The root is built under the Conversation's profile**, `homes/<id>/.claude`,
  emptied and made again as each session starts, the way the rest of the profile
  already is on Windows and a Mac. **Linux gains a `homes/<id>` directory** under
  the Data Directory for this: today its whole profile is a tmpfs over the
  server's home and one bind of the account, and a root has to exist somewhere
  to be bound. The Linux rendering binds the built `.claude` over `$HOME/.claude`
  inside the tmpfs.
- **The allowlist, and nothing outside it.** `.credentials.json` is *linked* —
  a hard link on Windows, a symlink on a Mac, a bind on Linux — so a login or a
  token refresh from inside lands in the account; where the agent replaces it
  rather than writing in place, the write-back carries it, on every platform —
  see the write-back decision below. `projects/` is
  joined read-write — junction, symlink, bind — because it holds Claude's
  per-Repo memory, keyed by the Repo's main checkout rather than the worktree,
  and because session-log discovery already globs there. This stage joins it
  unconditionally; stage 02 puts the switch in front of it. `settings.json` is
  written by Verkstead: `skipDangerousModePermissionPrompt: true`, plus the
  account's own `apiKeyHelper` and `env` copied over where the account's
  `settings.json` has them, because those are how an API-key login reaches the
  model. Nothing else of the account's `settings.json` — an allowlist rather than
  a denylist, because a denylist drifts every time Claude adds a key. Everything
  else under `~/.claude` is absent from the root: `plugins/`, `commands/`,
  `agents/`, `skills/`, `CLAUDE.md`, `history.jsonl`, and whatever else it grows.
- **`.claude.json` is copied and pre-seeded, and written back.** Copied rather
  than linked, so the pre-seeding is written into the session's copy and not
  straight into the account's file. Pre-seeded with a `projects` entry for the
  Repo's own path and for the Worktree, each `hasTrustDialogAccepted: true`, and
  with `bypassPermissionsModeAccepted: true`; Codex's launch line already
  pre-seeds its Worktree trust, which is the precedent. The write-back at
  session end stays exactly as ADR-0014 decided it — the human wants a re-login
  from inside a run to reach the account, and accepted that the trust entries
  reach it too. The identity check that drives the write-back already treats a
  copy as a replaced file.
- **The write-back runs on all three platforms.** Today it is Windows's alone:
  it was built for hard links, and `Closing` is `nothing` on Linux and a Mac
  (`sandbox/closing.rs`), because a bind and a symlink follow their target. A
  copy follows nothing on any platform, so without this a re-login inside a
  Linux or Mac run never reaches the account, and a credentials file the agent
  replaces through a symlink is lost with the profile. So the Unix renderings
  hand back the credentials file and the `.claude.json` copy the way the Windows
  rendering does, and the same identity check decides.
- **The cover over `.claude/skills` goes.** ADR-0011 bound an empty directory
  read-only over it to hide the account's own skills; a built root has nothing
  there to hide, and `/verkstead/skills` is unchanged.
- **The Windows boundary grants the root, not the account.** The read-write
  entry on the whole of `~/.claude` — 1,289 files on the reporter's machine, and
  the tree a first boundary walks — becomes an entry on the built root plus one
  on the `projects/` target; a hard link shares the file's own list, so the
  credentials file is reachable as it is today. `%APPDATA%`, `%LOCALAPPDATA%`,
  `TEMP` and the rest of the fresh profile are untouched.
- **Detection is unchanged.** The wizard still finds an account by `~/.claude`
  and `~/.claude.json` both existing, and a Profile still stores that pair. What
  changes is what a session is *given* of it.
- **`DISABLE_AUTOUPDATER` stays set.** A root of Verkstead's own is still not a
  place for an agent to install itself.
- **Not `--setting-sources` or `--settings`, not `--safe-mode`.** Flags leave
  the join in place and so leave the plugin-registry write and the settings
  leak open; `--safe-mode` disables the Repo's `CLAUDE.md`, skills and MCP,
  which a session needs.
- **Interactive on the pseudoconsole, as before.** `--print` is not the fix for
  the consent prompt; the settings key is.

## Proposed tasks (provisional)

1. **A `homes/<id>` on Linux.** The profile directory exists on all three
   platforms, made and emptied per session; the Linux rendering binds what is
   built there into the tmpfs home. AC: the Linux suite sees a built directory
   under the Data Directory; nothing the session writes under `$HOME` lands
   outside it but through a bind.
2. **The built `.claude` by allowlist.** Credentials linked, `projects/` joined,
   the rest absent, on all three renderings. AC: the account's `settings.json`,
   `plugins/` and `CLAUDE.md` are not visible inside; a login written inside
   appears in the account; the three boundary suites assert it.
3. **The written `settings.json` and the pre-seeded `.claude.json`.** AC: a
   fresh account with no `settings.json` still gets the bypass key; an account
   with `apiKeyHelper` keeps it and loses its `hooks`; the Repo and the Worktree
   read as trusted in the copy; the write-back carries the copy back at session
   end.
4. **The write-back on every platform.** The Linux and Mac renderings hand back
   a `Closing` naming the credentials file and the `.claude.json` copy. AC: on
   all three, a `.claude.json` changed inside reaches the account at session
   end; a credentials file replaced by rename inside reaches the account; a file
   still one with the account's is left alone.
5. **The `.claude/skills` cover removed**, and the Windows grant narrowed to
   the root. AC: the Surface names no entry on the account directory; the
   Windows suite's boundary is written over the root and the `projects/` target
   only.
6. **The docs.** CONTEXT.md's Agent Profile and Sandbox entries, the adoption
   doc's three platform sections and its NixOS `paths` example, `docs/design`
   where it says the pair is bind-mounted. AC: no document says the account is
   joined whole.

## Re-verify at start

- Assumes the profile is still built by `Sandbox::surface` in one place and
  rendered three ways, and that `account_inside` is the one list the sandbox and
  the wizard read.
- Assumes Claude Code still keys trust and memory by the Repo's main checkout,
  still reads `skipDangerousModePermissionPrompt` from `settings.json`, and
  still saves `.claude.json` by write-and-rename. Check against the shipped
  version before finalising the pre-seed.
- Assumes session-log discovery still globs `projects/` for `<session-id>.jsonl`.
- Assumes ADR-0014's write-back still decides by file identity rather than by
  remembering how the link was made.
- Check how Claude Code saves `.credentials.json`. A bind over a single file
  refuses a rename onto it, so if a token refresh saves by rename, Linux copies
  the file in rather than binding it and leaves the rest to the write-back.
