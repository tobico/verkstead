# 01. One Author, end to end

## What to build

Turn the single `git_author` in `config.yaml` and `github_token` in
`secrets.yaml` into a list of Authors, and move **every** reader of the old two
onto it — with nothing on screen changing.

The shape in the files (ADR-0025):

```yaml
# config.yaml
authors:
  - id: 3f2c…-uuid
    display_name: Work        # optional; falls back to the git name
    name: Tobias Cohen
    email: tobi@tobico.net
```

```yaml
# secrets.yaml
author_tokens:
  3f2c…-uuid: ghp_…
```

The tokens are keyed by uuid the way `mcp_headers` is keyed by server name, and
they follow the settings module's own contract. Both files are read at the
moment they are needed, so a save reaches the next session without a restart.
An absent, empty or unparseable key configures nothing rather than refusing to
start, and blanks are nothing. Name the secrets key as reads best beside
`mcp_headers`; the key above is a suggestion. There is no signing key.

**The first start migrates.** Where `git_author` or `github_token` is present
and `authors:` is not, the server mints one Author with a fresh uuid and no
display name, writes the list to `config.yaml` and the token under that uuid to
`secrets.yaml`, and the single keys are gone from both files afterwards. Every
other key in both files survives the rewrite, including whatever the page never
draws (see `Config::keeping_what_the_page_never_drew`). Run it once at the
serve, before anything reads an Author. Task 02's Repo migration needs to know
which uuid was minted, so let the migration say so.

**The API**: `Config` answers *the Author with this uuid* and *every Author* in
order, and `Secrets` answers *the token of this uuid* and replaces one uuid's
token while leaving every other secret as it was. "The first Author" is just the
head of that list. The `Author::configured` helper (both halves filled) carries
over to an Author from the list.

**Every reader moves onto the one Author (the first in the list) for now.**
Task 03 moves them onto the Repo's Author. Today the readers are:
- the sandbox's `git_author` and `github_token`, set in `for_conversation`;
- `for_login`, which gets **no git author at all** from here on, since a login
  has no Repo and commits nothing (decided in planning; it already gets no
  token);
- the cuts' `Author::configured(config.git_author())` in `continuing.rs` and in
  `conversations.rs` (grilling and adoption);
- `create_repo` in `ui.rs`;
- sharing's `publishing` in `ui.rs` (the gist, its commit, and the token handed
  on to the link comments);
- `Gh::token`;
- onboarding's step condition and prefill (*an Author configured* is the third
  step's condition, as *a git author* was);
- the settings view and its save.

Grep for `git_author()` and `github_token()` again before starting.

**The settings view and save carry the one Author by uuid.** The render crate's
settings view and edit types get the Author's uuid, display name, git name,
email and the token's saved state, and the save writes into that Author. A save
with no Author yet (a fresh install) creates the first one with a fresh uuid.
Neither the Git pane nor the wizard's git step changes on screen: they keep
their fields, their warnings, their token verification and their words. The
display name field is task 04's.

## Acceptance criteria

- [ ] A hand-written `config.yaml` and `secrets.yaml` holding the old
      `git_author` and `github_token` read, after one start, as a one-Author
      list with the token under its uuid. The old keys are gone from both files,
      and every other key in them is as it was.
- [ ] An absent, empty or unparseable `authors:` is no Authors. A session then
      gets no `user.name`/`user.email` and no `GH_TOKEN`, so git's own "tell me
      who you are" stands, and the cuts refuse as a missing git author refuses
      today.
- [ ] Saving a token under one uuid leaves every other secret (other tokens,
      `session_account_password`, `mcp_headers`) as it was.
- [ ] The Git pane and the wizard's git step save the one Author and verify its
      token as today. The web suite passes unchanged in what it asserts on
      screen.
- [ ] A Profile's login sandbox sets no `GIT_CONFIG_*` author pairs.
- [ ] The sandbox suite's test over the names a session is given still
      enumerates the same names.
