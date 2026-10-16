# 03. The session gets its Repo's Author

## What to build

Everything a session is given, and everything Verkstead itself does on a Repo,
comes from **that Repo's Author** rather than from the first one in the list.

- **The sandbox.** `Sandbox::for_conversation` composes the `GIT_CONFIG_*` pairs
  (`user.name`, `user.email`) and `GH_TOKEN` from the Conversation's Repo's
  Author. That covers every platform's rendering, because they share the one
  `Surface`, and a Conversation Terminal, which is that sandbox `shelled`. The
  list of names a sandbox sets does not change. A Repo with no recorded Author
  gets no author and no token, as an installation with none does today. Stage
  02 turns that into a refusal. A Companion Repo contributes nothing of its own
  Author, because a sandbox has one environment.
- **The cuts.** The clearing of an inherited `.tasks/` at a grilling's cut, at
  adoption and at a roadmap stage's carry-on commits as the Conversation's
  Repo's Author. A cut on a Repo whose Author lacks a name or an email is
  refused, or halted with a Notice, exactly as a missing git author is today.
  The sentences name the Repo's Author where there is more than one Author; see
  `docs/copy.md`.
- **The server's `gh`.** `Gh` reads the token by the Repo the call runs in, not
  from one configured value. Every call about a Repo already passes the Repo's
  path; resolve that to the registered Repo and its Author, through the store
  or by the caller handing over the Repo, whichever fits the code. A companion's
  pull request read runs as the companion Repo's Author. The stack-extension
  check (`stacks::missing` → `Gh::extension`) runs as the Conversation's Repo's
  Author. A call about no Repo, which is `authenticates_as` verifying a token
  just typed, still authenticates as the token it was given.
- **Sharing.** Publishing a Conversation (the gist, its commit, and the link
  comment on each pull request) runs as the Conversation's Repo's Author: its
  token and its name.
- **Create repo** keeps committing as the one Author until task 05 adds the
  pick. Stage 02 moves its `gh repo create` onto the picked Author.

This is proven with the second Author added by hand in `config.yaml` and
`secrets.yaml`, and the Repo's row recorded directly. Task 04 builds the screen
for adding one.

## Acceptance criteria

- [ ] Two Repos under two Authors run two sessions whose environments carry
      different `user.email` values and different `GH_TOKEN` values, and
      neither carries the other's.
- [ ] A Conversation Terminal on each Repo carries the same as its session.
- [ ] A cut on a Repo whose Author has no email is refused by name, exactly as
      a missing git author refuses one today.
- [ ] A server-side pull request read on a Repo runs `gh` with that Repo's
      Author's token, proven with a stand-in `gh`. Verifying a typed token
      still runs as the typed token.
- [ ] Sharing a Conversation on the second Author's Repo publishes and comments
      with the second Author's token, proven with a stand-in `gh`.
