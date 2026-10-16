# 05. The settings copied at boot

## What to build

**The Annex has no settings of its own** (ADR-0024). At every boot, alongside
the git author and GitHub token that stage 03's `InTheBox::commit_as` already
hands in, the Keeper copies into the Annex's `config.yaml`:

- `instructions`
- `mcp_servers`, with each server's header **names**
- `at_once`
- `cleanup`
- `ignored_comments`, `conflict_resolution` and `share_on_done`. These were
  added after the brief (Set 30 Q3): a Conversation in the Annex runs Wrapping
  and Done there, so they are about its work too.

It also copies into the Annex's `secrets.yaml`:

- `mcp_headers`, the header **values** for those servers, beside the GitHub
  token. A server that arrived without its values would be declared in the
  Annex and refused by its far end.

**Everything else stays the Annex's own**, because it is about the Annex's
machine: its Repos, its Members, its sandbox binds, `session_path`,
`languages`, `kept_by`. A key the copy does not own is carried through
untouched, which is how `commit_as` already treats the files.

**A copy of the Keeper's values, not a merge.** An entry the Keeper no longer
has is gone from the Annex too (an MCP server removed on the Keeper, its
headers with it). A file that already says what the copy would write is not
written, so an unchanged boot touches nothing. Files written are handed back
to the service user the way `commit_as`'s are.

Generalise the existing hand-in rather than adding a second one beside it: one
step of the link, reading the Keeper's settings at that moment and writing both
files, provable over the stub `wsl.exe`.

**The proof is end to end on Windows** for the two cases the brief named. A
change on the Keeper's Instructions pane reaches the next session in the Annex
after a restart of the Keeper. An MCP server declared on the Keeper with an
auth header is reached from a session in the Annex with that header sent (a
throwaway HTTP endpoint that logs its request headers is enough).

## Acceptance criteria

- [ ] After a Keeper restart, the Annex's `config.yaml` carries the Keeper's
      Instructions, MCP servers, at-once limit, Cleanup, ignored comments,
      conflict resolution and share-on-done, and its sandbox binds and Repos
      are as they were.
- [ ] Its `secrets.yaml` carries the Keeper's `mcp_headers` beside the GitHub
      token, and a server removed on the Keeper is gone from both files.
- [ ] A changed Instructions text reaches the next session in the Annex after a
      restart, and an MCP server with an auth header is reached from the Annex
      with that header sent, both on Windows.
- [ ] A boot with nothing changed writes neither file (covered over the stub).
