# 03. The written `settings.json`

## What to build

On all three platforms, the built root gets a `settings.json` that Verkstead
writes as each session starts. It is not linked and not written back. It holds:

- `skipDangerousModePermissionPrompt: true`, which Claude Code 2.1.268 reads
  from user settings. This is what stops a fresh account parking for ever on
  the bypass-permissions consent (finding 9).
- the account's own `apiKeyHelper` and `env`, copied over where the account's
  `settings.json` has them, because those are how an API-key login reaches the
  model.

Nothing else of the account's `settings.json` goes in. This is an allowlist
rather than a denylist, because a denylist drifts every time Claude adds a key:
no `hooks`, `enabledPlugins`, `permissions`, `statusLine` or anything else. An
account with no `settings.json`, or one that does not parse, still gets the
bypass key.

**Not `--setting-sources`, `--settings` or `--safe-mode`,** and not `--print`:
the session stays interactive on the pseudoconsole.

## Acceptance criteria

- [ ] A fresh account with no `settings.json` gets one inside holding the bypass
      key, on all three platforms' suites.
- [ ] An account whose `settings.json` has `apiKeyHelper`, `env` and `hooks` gets
      the first two inside and not the third.
- [ ] The account's own `settings.json` is unchanged after a session.
