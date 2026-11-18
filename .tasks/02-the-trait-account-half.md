# 02. The trait, account half

## What to build

Finish moving the server behind the trait task 01 introduced. Replace every
remaining branch on `AgentType` or `store::Account` in non-test server code with
a call to it. This is the account and Built Root side:

- **account layout and mounts**: `account_inside`, `kept_in` /
  `account_in_home`, Claude's config-file copy and config closing, `login_of`,
  `login_at` and `login_inside`
- **Built Root writing**: the per-harness `Root` constructors and every branch
  on the private enum in `sandbox/root.rs`. That covers `built`,
  `furnished_in`, `written` (the settings and the declared MCP servers in each
  harness's own format: settings.json, config.toml, opencode.json, agy's
  settings), the instructions file name (CLAUDE.md / AGENTS.md / agy's),
  `login_copied`, Claude's trusted-dirs `config`, `unshared_in`, the joined
  store and files, `entries` and `synced`. Shape the root-writing call as the
  later `write-configuration` export will see it (stage brief): it takes the
  account's configuration *as found, with the account's own servers already
  taken out* (ADR-0011's strip-and-never-write-back stays the host's) plus the
  attached servers, and answers with the files the Built Root holds and the
  session-only settings. The config write-back (`merged_back`) stays the host's.
- **login**: the `Speaking` table in `logins.rs`, whether a harness logs in at
  all, and the login-probe gate in `asked()`
- **transcript search**: `Search::of` and the per-harness locators, and
  `carried()`, which finds a prior session's log to carry. Parsing in the render
  crate does not branch on the type and stays where it is.
- **memory roots and mirroring**: `mirroring/memory.rs`, `peer/memory.rs`,
  `mirrored_of` and `mirrored`
- **Profile validation**: `server/profiles.rs`'s `broken()` and `holds_login`
- **onboarding detection**: the `Dependency` → harness mapping, and finding
  accounts in the home

Behaviour stays byte-for-byte. The suites are the proof, and they are not
edited to pass. The store's own conversions (`word`, `read`, `nowhere`, the
binding) and the wire conversions belong to task 03.

## Acceptance criteria

- [ ] No `match`, `if let` or `matches!` on `AgentType` or `store::Account`
      remains in non-test server code. The only exception is the lookup from
      type to implementation, which task 03 replaces.
- [ ] The sessions, logins, transcript, mirroring, peer-memory and onboarding
      suites pass unchanged.
- [ ] A Built Root for each of the five harnesses is identical, file for file,
      to what it was before the move. Prove it with a test that compares them,
      if no existing test does.
