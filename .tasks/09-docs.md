# 09. Docs

## What to build

Everything written down that says Windows runs no sessions is now wrong, and
what replaces it is the unsandboxed state — said where a Windows reader will
find it, and said as what the product *is* today rather than as a promise.

- **README** — the paragraph that says sessions run on Linux and on a Mac and
  that "Windows has everything but those". Windows runs them now, over no
  boundary at all until stage 03.
- **`docs/adoption.md`** — the Windows section's "Sessions do not run on
  Windows", which quotes the refusal sentence that no longer exists. What a
  Windows human needs to know is that sessions run, that they run with their own
  account's reach, that the workbench says so on every one, and that a later
  stage closes it.
- **`docs/design/verkstead.md`** — the revision note that ends "and Windows is
  still without sessions".
- **`docs/development.md`** — whatever it says about what a Windows checkout can
  and cannot run.
- **`CONTEXT.md`'s Sandbox term**, which says nothing about Windows today. It
  gains the unsandboxed state here — the term is written as what the product is,
  so it moves when the stage moves it rather than ahead of it, and stage 03
  takes the same sentence back out.
- **`CONTEXT.md`'s Terminal term**, which this stage makes wrong in four ways at
  once. It says a Terminal is the server user's login shell, `/bin/sh` where
  there is no usable one, inside the worktree's dev shell and inside the
  Conversation's Sandbox — and on Windows it is `pwsh` or Windows PowerShell,
  there is no passwd entry to read, `nix develop` is skipped by Platform, and
  there is no Sandbox at all until stage 03. The term says what a word means
  rather than what one platform does, so what it gains is the Windows answer
  beside the Unix one rather than a second entry.

ADR-0014 already says the why and is not repeated. Nothing here invents a new
claim: every sentence written is one the eight tasks before it made true.

## Acceptance criteria

- [ ] Nothing left in the docs says Windows runs no sessions, and no document
      quotes the removed refusal sentence.
- [ ] The unsandboxed state — the agent runs with the human's own account's
      reach until stage 03 — is written where a Windows reader is already
      looking: the README, the adoption Windows section, and the design note.
- [ ] `CONTEXT.md`'s Sandbox term says what a Windows session gets today, and
      its Terminal term names the shell a Windows terminal opens on beside the
      passwd one, in the same entry.
