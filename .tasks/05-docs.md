# 05. The docs

## What to build

Bring every document that says the account is joined whole into line with the
built root as tasks 01 to 04 landed it:

- **CONTEXT.md**: the Sandbox entry (it lists "the Agent Profile's pair at
  `~/.claude` and `~/.claude.json`" as read-write, and says nothing stands where
  the account's own skills would be) and the Agent Profile entry (it says the
  pair is bind-mounted, and that mounting it keeps accounts separate). Define
  the built root in the project's vocabulary.
- **docs/adoption.md**: the table row naming the claude pair, the skills line,
  the NixOS `paths` example naming `/home/you/.claude` and the paragraph
  explaining it, the Mac section ("the account linked into it"), and the
  Windows section (the grant on the account, the entry refusing its skills, and
  the junction and hard-link paragraph). Check whether a Claude account still
  has to be in NixOS `paths` now that only the credentials file and two
  `projects/` entries are reached.
- **docs/design/verkstead.md**: where the profile's pair is bind-mounted, the
  read-write list, and the empty directory bound over `~/.claude/skills`.
- **ADR-0011 and ADR-0014**, under *Amended: the account is built rather than
  joined*:
  - name `apiKeyHelper` and `env` as the keys copied into the written
    `settings.json`;
  - record that the `.claude.json` write-back is a key-level merge, with
    `mcpServers` stripped from the copy, and why;
  - record that `bypassPermissionsModeAccepted` is not seeded;
  - correct ADR-0014's "tmpfs" for the Linux home, which is an empty directory
    bubblewrap makes over the server's HOME.

## Acceptance criteria

- [ ] No document says the account is joined, mounted or granted whole.
- [ ] The NixOS example and its explanation match what a session reaches.
- [ ] Both ADR amendments name the settings allowlist, the merge write-back and
      the MCP stripping.
