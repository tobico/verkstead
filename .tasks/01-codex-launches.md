# 01. The `codex` type launches

## What to build

A Codex Profile that runs the real `codex`. Today the launch line is one
command for every agent type with claude's name in it, the per-type flag
mapping is empty for Codex, and the Profile form draws Codex's home row but
offers no way to choose the type — so a Codex Profile can only be written over
the API and would launch claude if it were.

Three things, one slice:

- **The binary is the agent type's.** Codex where claude goes for a Profile of
  that type, without disturbing the way the test suite stands a stub program
  where the agent goes — the stub substitution is what proves a session's
  output reaches the Timeline while it runs, and it has to keep working for
  every type.
- **The launch line codex actually takes.** The prompt as the one positional,
  the Pairing's model as `-m`, `--dangerously-bypass-approvals-and-sandbox`
  (Verkstead's sandbox is the boundary and codex's own will not start inside
  bwrap), `--no-alt-screen` so the Capture stays a record somebody can read,
  and **no session id** — codex takes none, which is why its log is found
  rather than named in task 03. Claude's line does not move.
- **What the account needs said on the line rather than written into it.**
  Codex reads `-c key=value` overrides from the command line, which is how the
  home is configured without Verkstead writing into a directory that belongs to
  the human's account: the credential store file-backed
  (`cli_auth_credentials_store = "file"`, since there is no keyring inside the
  sandbox), and the Worktree pre-seeded as trusted — some versions still show
  the trust prompt despite the bypass, and a session stopped at a prompt is a
  run waiting on nobody.

And the form: the agent type becomes something the human picks when adding a
Profile, drawing that type's own path rows under it. Codex is offered from this
stage on, which is what the rule about a type that cannot launch being a lie in
a picker was waiting for.

**The home is left as the account keeps it.** ADR-0011's rule about binding an
empty directory over a backend's own skill-discovery path does not apply to
Codex: that path is inside `~/.codex`, which is the whole of what a Codex
Profile names, and covering it would hide the skills codex itself ships. The
ADR says so as of this stage's plan.

Verified against codex 0.149.0: the flags above all exist, `--yolo` does not
any more, and the sandbox's PATH already reaches the host's `codex`.

## Acceptance criteria

- [ ] A Codex Profile is saved from the form — the type picked there rather
      than over the API — paired with a Conversation, and launched; its Capture
      shows codex running under that Profile's home.
- [ ] The line carries the bypass flag, `--no-alt-screen`, the model as `-m`
      and the prompt as the positional, with no session id; Claude's line is
      unchanged and the suite's stub agents still read the line they read
      today.
- [ ] The credential store is file-backed and the Worktree trusted from the
      launch line alone, with nothing written into the Profile's own directory
      by Verkstead.
