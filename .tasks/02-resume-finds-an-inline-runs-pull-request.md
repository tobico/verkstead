# 02. Resume finds an inline run's pull request

## What to build

Resume on an inline Implementing Conversation looks for the pull request
before spending a session — the inline counterpart of what `roadmap_again`
already does for roadmaps. This is also how a Conversation whose PR was opened
by hand (the wrap-up halt's own advice: "open the PR by hand, and resume")
actually gets found, which today it never is.

`inline_again` currently launches a fresh implementing session
unconditionally. Instead it first asks `gh` for the PR on the Conversation's
branch (the same `github::pull_request` call `wrapping::opened` makes, off the
runtime's threads), and decides by what comes back:

- **A PR found** — go straight to `wrapping::opened` (which will find it
  again, record it, and move the Conversation to Wrapping). No session spent.
- **`Trouble::NoPullRequest`** — the work genuinely is not on a PR yet: launch
  the fresh implementing session exactly as today.
- **Any other `Trouble`** (no `gh`, not logged in, no remote, refused) — halt
  naming it, without launching anything: a session started then could only
  dead-end on the same missing thing, and the halt is what tells the human
  from their phone.

Settled with the human as Q2 of the grilling; the three-way split by
`Trouble` variant was settled reading `github.rs`, which already distinguishes
`NoPullRequest` from `gh` being unable to answer.

## Acceptance criteria

- [ ] Resume on an inline Implementing Conversation whose branch has a PR
      moves it to Wrapping without launching a session
- [ ] Resume where GitHub has no PR for the branch launches a fresh
      implementing session, as before
- [ ] Resume where `gh` cannot answer (absent, logged out) halts with a Notice
      naming the reason, and launches nothing
