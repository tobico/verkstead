# 01. An inline run ends on its pull request

## What to build

An inline implementation session carries its own branch to a pull request, and
Verkstead finds that PR and moves the Conversation to Wrapping — the same
ending a backlog's finish step already has.

Two halves, settled with the human:

- **The implementing skill opens the PR itself** (the bundled skill under
  `crates/server/skills/implementing/`). Its current ending — "Do not push,
  and do not open a pull request. Getting the branch reviewed is a step of its
  own that Verkstead runs after this one" — promises a step that does not
  exist. Replace it with the finish sequence the next-task skill's "Then get
  the branch reviewed" section uses, word-shape and all: follow the target
  repository's own `docs/agents/git-workflow.md` review process where it has
  one, fall back to `git push -u origin HEAD` plus a **draft** PR otherwise,
  no gate and no approval anywhere. The wording must also cover a resumed
  session that finds the work already committed by a previous session: verify
  it and carry it to the PR rather than treating "nothing to build" as
  "nothing to do".
- **The runner asks GitHub about it.** `follow_inline`'s success arm (session
  ended well and landed commits) currently just returns; it must call
  `wrapping::opened`, passing the session's Timeline Event so a halt written
  there carries the session's tail — exactly as the finish step's arm in
  `work`/`see_out` does. `wrapping::opened` already does the rest: records the
  PR and moves the Conversation to Wrapping in one transaction, starts the
  wrap-up watchers, and halts with a Notice when no PR can be found.

The choice of "the implementing session opens it" over "a separate finish
session" was deliberate (staging already works this way for roadmaps; no extra
session spent; the context that built the work writes the PR body) — do not
reintroduce a separate finish session.

## Acceptance criteria

- [ ] The implementing skill instructs the session to end by pushing and
      opening a draft PR, repository's git-workflow first, and says what a
      resumed session with already-committed work does
- [ ] An inline session that ends cleanly having committed work leads to
      `wrapping::opened`: the PR is recorded and the Conversation is Wrapping
- [ ] An inline session that ends cleanly with commits but no PR on the branch
      halts with the existing "no pull request found" Notice, and the failure
      arms (ended badly, committed nothing) behave exactly as before
