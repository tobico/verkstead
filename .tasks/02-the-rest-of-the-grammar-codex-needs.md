# 02. The rest of the grammar Codex needs

## What to build

Three things Codex does that a plugin's `agent` provision cannot yet say,
added to the manifest grammar (`plugins/manifest/agent.rs` in the server) and
to the plugged harness, and proven on the stub harness.

1. **A resume subcommand with no `{session}`** (Set 1836, Q4). Codex's line is
   `codex resume -m <model> … <id> <prompt>`: the word after the binary, the id
   a positional in front of the prompt. The trait's `Resume::Subcommand`
   already supports that when its words hold no `{session}`; the manifest
   reader currently refuses it ("refuse a resume with no session"). Allow it,
   meaning the id goes in front of the prompt, and keep refusing a resume that
   is a flag without `{session}`. The subcommand's `also` (resume-only tail
   words) must be expressible too.
2. **How each memory directory syncs** (Set 1836, Q5 — data, not an export). A
   plugin harness's `synced` answers nothing today ("a plugin harness's memory
   does not cross a link"). Each declared memory path gains a word naming one of
   the host's existing rules — the `Whose` variants in `sandbox/root.rs`:
   everything under it, the rollouts naming this Worktree, the session
   directories called by the Conversation's session ids, a database and its
   `-wal`/`-shm`. Codex will declare `memories` whole and `sessions` by
   Worktree. The plugged harness answers `synced` from it, so the mirroring and
   peer memory code (which call `root.synced(...)`) carry a plugin harness's
   memory as they carry a built-in's.
3. **What a mirror of the account holds** (Q5). The plugged harness's
   `mirroring` copies its configuration files raw; Codex's holds only the keys
   that reach a model. Make a plugin harness's mirror hold what its
   `write-configuration` answers when handed the account's configuration with
   no servers — which is exactly what Codex's in-process mirror is
   (`toml_carrying` with no servers).

Write the new words into `docs/plugins.md` where the agent provision is
described.

## Acceptance criteria

- [ ] `verkstead plugin check` accepts a resume subcommand with no `{session}` and a memory path with each sync word, and refuses an unknown sync word naming the field
- [ ] A stub session resumed through a subcommand without `{session}` is launched with the id in front of the prompt
- [ ] A stub Profile's memory, declared synced, reaches a device it is not bound on and comes back, in the mirroring suite
- [ ] A stub account's mirror holds what its `write-configuration` answers with no servers, not the raw file
