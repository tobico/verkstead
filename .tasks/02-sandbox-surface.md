# 02. The sandbox surface

## What to build

The orchestrator can run a command inside a bwrap sandbox built for a
Conversation, and what that command can see is exactly what the design says and
nothing else. Evolved from `tobico-scripts/bin/sandbox`, which is the working
reference — but narrowed: today's script binds the whole of `~/src`
read-write, and that blanket bind is the thing this replaces.

The surface, per the design:

- **read-write** — the Conversation's worktree; the Repo's common `.git`
  directory; the grilling Profile's claude pair, mounted at `~/.claude` and
  `~/.claude.json`
- **read-only** — `/nix` and the system paths, `~/.gitconfig`, the gh config
- **tmpfs** — `/tmp`, and everything else in HOME simply absent
- **network** — full, unrestricted. The filesystem is the boundary. Leave the
  seam for a proxy allowlist later, but do not build one.

Per-repo extra binds come from sandbox configuration, composed from a global
set plus a per-repo set, so a repository that needs a cache directory can say
so without every repository getting it.

Nix dev-shell autodetection carries over from the script and keeps its
reasoning: wrap the command in `nix develop` only when a shell attribute
actually evaluates, because a `flake.nix` alone is not enough and `nix develop`
errors out when none of the attributes it falls back through exist.

What proves this is a probe — a command run inside the sandbox that reports
what it can reach — rather than a reading of the flags. The flags are what is
being tested; a test that asserts them asserts itself.

## Acceptance criteria

- [ ] A probe run in a Conversation's sandbox can write its worktree and the
      Repo's `.git`, read `/nix`, `~/.gitconfig` and the gh config, and write
      neither of the last two
- [ ] The probe finds the Profile's pair at `~/.claude` and `~/.claude.json`,
      and finds the rest of HOME empty
- [ ] The probe cannot reach any path under the host's `~/src` that is not the
      Conversation's own worktree
- [ ] The probe reaches the network
- [ ] A repo with a dev shell runs its command under `nix develop`; a repo with
      a `flake.nix` and no dev shell, and a repo with no flake at all, both run
      it directly
- [ ] Per-repo extra binds compose over the global ones and appear in the
      sandbox
