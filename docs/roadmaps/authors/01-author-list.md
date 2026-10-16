# 01. The author list

## Goal

A second Author, with a token of its own, is added on the settings page's Git
section; a Repo opened afterwards is recorded as its; and every session on that
Repo commits as that name, pushes with that token, and has the first Author's
token nowhere in its sandbox. An installation that never adds a second Author
sees nothing change: the Git section reads as it did, the single `git_author`
and `github_token` have become one Author in the files, and every registered
Repo is recorded as that one.

## Decisions in force

All from [ADR-0025](../../adr/0025-a-repo-has-an-author.md) and the **Author**
term in `CONTEXT.md`; the why is there and is not repeated beyond what a task
needs.

- **An Author is a git name, an email and an optional GitHub token, keyed by a
  uuid, with an optional display name that falls back to the git name.** No
  signing key. The uuid is what a Repo records; the display name is for rows
  and pickers and may change.
- **`authors:` in `config.yaml`, tokens in `secrets.yaml` under the uuid** — the
  split the token and the MCP header values follow today, both files read at the
  moment they are needed so a save reaches the next session without a restart,
  and an absent, empty or unparseable key configuring nothing rather than
  refusing to start. The settings module's own contract.
- **The first start moves the single keys into the list.** `git_author` and
  `github_token` become one Author with a fresh uuid and no display name, and
  the single keys are gone from the files afterwards. One shape in the files;
  leaving the old keys as an unnamed Author was turned down.
- **Every Repo records its Author in the store**, beside the `repos` row rather
  than as a column on it — the table is STRICT and the side-table shape is what
  `unregistered_repos` already uses. The migration records every registered
  Repo as the migrated Author's. Registering a Repo with one Author configured
  records that Author silently.
- **With one Author nothing on screen changes.** The Git section draws its
  author dropdown and **Add author** only once there are two to choose between;
  until then the fields are the one Author's and the page reads as today.
- **The Git section keeps its shape.** A dropdown at its head picks which
  Author the fields — display name, git name, email, the write-only token with
  its last four characters, the account GitHub verified it as — are about, with
  **Add author** at its foot the way the Repo dropdown carries **Open repo** and
  **Create repo**. Cards per Author and a list opening a pane apiece were both
  turned down. Token verification is per Author, on save, naming the login and
  any scope it lacks, as today.
- **A session is handed its Repo's Author.** The `GIT_CONFIG_*` pairs and
  `GH_TOKEN` the sandbox sets come from the Repo's Author rather than from the
  one configured value, on every platform's rendering and for a Conversation
  Terminal as much as a session. The list of names a sandbox sets itself does
  not change.
- **Every server-side `gh` call on a Repo authenticates as that Repo's
  Author's token.** The one reader of the configured token already runs each
  call inside a Repo, so the Repo is what it reads by. A call about no Repo —
  verifying a token just typed — authenticates as the token it was given.
- **Everything Verkstead commits on a Repo's behalf is by that Repo's Author**:
  the clearing of an inherited `.tasks/` at a cut, and the first commit of a
  created repository. A cut on a Repo whose Author lacks a name or an email is
  refused the way a missing git author is refused today.
- **The wizard still asks for one Author**, and nothing about onboarding's
  objective changes: *an Author configured* is the third step's condition as
  *a git author* was.

## Proposed tasks (provisional)

1. **Authors in the settings files.** `authors:` as a list keyed by uuid with
   display name, git name and email; tokens under the uuid in `secrets.yaml`;
   the first-start migration of the single keys; a `Config` and `Secrets` API
   that answers *the Author with this uuid* and *every Author*.
   - A hand-written `config.yaml` with the old single keys reads, after one
     start, as a one-Author list, and the old keys are gone from both files.
   - An absent, empty or unparseable `authors:` is no Authors, and git's own
     "tell me who you are" stands as it does today.
   - Saving a token under one uuid leaves every other secret in the file as it
     was.

2. **The Repo's pick, in the store.** A side table recording each Repo's Author
   uuid; the migration writes every registered Repo against the migrated
   Author; registering with one Author configured records it silently; the
   Repo views carry the uuid.
   - A fresh registration with one Author configured is recorded as that
     Author's with nothing asked.
   - Reviving an unregistered Repo keeps the Author it had.
   - A Repo with no recorded Author and more than one configured is a fact the
     view can state, for 02's refusals to act on.

3. **The session gets its Repo's Author.** The sandbox's surface composes the
   git configuration and `GH_TOKEN` from the Conversation's Repo's Author; the
   cuts commit as that Author; the server's `gh` reads the token by the Repo
   the call runs in.
   - Two Repos under two Authors run two sessions whose environments carry
     different `user.email` values and different `GH_TOKEN` values, and neither
     carries the other's.
   - A cut on a Repo whose Author has no email is refused by name, exactly as
     a missing git author refuses one today.
   - A server-side pull request read on a Repo runs `gh` with that Repo's
     Author's token, proven with a stand-in `gh`.

4. **The Git section with its switcher.** The settings view carries every
   Author; the dropdown at the section's head picks which the fields are about,
   drawn only once there are two; **Add author** at its foot opens the same
   fields over a new uuid; the token edit and its verification are per Author.
   - With one Author the pane reads as it does today, no dropdown drawn.
   - Adding a second Author draws the dropdown with both, the display name
     falling back to the git name, and saving a token under the second verifies
     and names that token's login.
   - The wizard's git step saves the one Author and is otherwise unchanged.

## Re-verify at start

- Assumes the settings module still keeps `git_author` as a `GitAuthor` on
  `Config` and `github_token` on `Secrets`, written whole by `save_config` and
  `save_secrets`, and that the MCP header values are still keyed by server name
  in `secrets.yaml` — the pattern the uuid-keyed tokens copy.
- Assumes the sandbox still composes its environment in one `Surface`, setting
  `GH_TOKEN` and the counted `GIT_CONFIG_*` pairs from values the Sandbox was
  built with, and that the sandbox suite's test over the names a session is
  given still enumerates them.
- Assumes the one `Gh` reader of the configured token still takes the Repo path
  on every call about a Repo, and that `authenticates_as` still takes a bare
  token.
- Assumes `repos` is still STRICT with `unregistered_repos` beside it and that
  `crates/store/src/migrations.rs` is where a one-time rewrite of existing rows
  goes.
- Assumes the Git section is still one pane under `web/src/settings/Git.tsx`
  with the wizard's step beside it under `web/src/setup/`, both reading one
  settings query, and that the Repo dropdown's listbox still carries action
  rows behind a rule, which is the control **Add author** copies.
- Check whether anything else reads `config.git_author()` by then — cuts,
  Create repo, onboarding, the settings view — and move each onto the Repo's
  Author or the first Author as its context says.
