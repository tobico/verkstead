---
name: reviewing
description: Review a branch that is already on a pull request and raise what it finds as one Question Set. Use when a session has been dispatched to review work, and changes none of it.
---

Review the branch this worktree is on, and raise what you find for the human to
decide about. **You review and you do not fix.** Nothing you find is changed by
this session.

You are the first thing to see this work whole. The sessions that wrote it each
saw one task and none of them saw the branch, and you have none of their
context — which is the point. Read it as somebody who has to live with it.

The branch is already pushed and already has an open pull request. There is
nothing to create, nothing to switch to, and nothing to open.

## 1. Read the work

Read the whole diff first, before you form an opinion about any part of it:

    gh pr diff

Then go and read what it landed in. A diff shows what changed and hides what it
changed *around* — the callers, the sibling module doing the same job a
different way, the test that should have caught this and does not.

Read what the repository says about itself, too: its `CLAUDE.md` or
`AGENTS.md`, the docs it keeps for agents, the conventions its neighbouring code
actually follows. The work is meant to look like it belongs here.

The Brief and the handoff in your prompt say what the work was *for*. Review
against those rather than against what you would have built.

## 2. What is worth raising

The seams are where this session earns its context. A session per task cannot
see across tasks, so look hardest at what only shows up from here:

- **Correctness** — the bug, the unhandled case, the thing that is wrong.
- **Seams between the pieces** — two tasks that solved the same problem twice,
  an abstraction introduced by one and ignored by the next, a half-done rename.
- **Drift from what was settled** — where the branch quietly decided something
  the handoff had already decided differently.
- **Tests that do not test** — the assertion that passes whatever the code does,
  the case nobody covered.
- **What is now stale** — docs, comments and names that describe the code as it
  was before this branch.

Raise what is worth a human's decision, and nothing else. Style you would have
done differently, a name you would have picked, a refactor nobody asked for —
these cost a human a decision each and buy nothing. **If you would not defend it
in a review, do not raise it.**

## 3. Change nothing

No edits, no commits, no pushes, no `gh` command that writes anything. Leave the
worktree exactly as you found it.

Fixing is a different session's job. What you raise, the human accepts or
declines, and each finding they accept is dispatched as its own fix session with
your words as its brief — so a finding has to stand up on its own, without you
there to explain it.

## 4. Raise it as one Question Set

One Set, one Question per finding, and a `review` block that tells Verkstead
which Option means *fix it*:

```yaml
title: Review of the rate limiter branch
preface: |
  Three things worth a decision. Everything else looks right to me.

  Answering **fix it** dispatches a session for that one finding alone;
  **leave it** dispatches nothing and I will not raise it again.
questions:
  - label: Q1
    text: |
      The window counter is never reset between windows, so a client that goes
      quiet for an hour is still refused. `window.rs` counts from the first
      request and nothing clears it.
    options:
      - n: 1
        text: Fix it
        recommended: true
      - n: 2
        text: Leave it
  - label: Q2
    text: |
      `limits.rs` and `window.rs` each grew their own clock. Two now, and the
      tests pin both.
    options:
      - n: 1
        text: Fix it
      - n: 2
        text: Leave it
        recommended: true
review:
  findings:
    - fix: Q1.1
      what: |
        `crates/limiter/src/window.rs` — `Window::count` accumulates for the
        life of the process and is never reset when the window rolls over, so a
        client that exceeds the limit is refused for ever rather than for the
        window. Reset it as the window rolls, and cover the roll-over in the
        tests beside `counts_requests_in_a_window`.
    - fix: Q2.1
      what: |
        `crates/limiter/src/limits.rs` and `window.rs` each hold their own
        notion of now. Collapse them onto one clock — `window.rs` has the
        better of the two — and update the tests that pin the other.
```

- **One Question per finding**, and every one of them answerable: two Options,
  one meaning fix it and one meaning leave it. Recommend the one you would take.
- **The Question is what the human reads**, on a phone, deciding. Write it as
  prose that says what is wrong and why it matters — not as a patch.
- **`what` is what the fix session is told**, and it is the only thing it gets
  from you. Write it for a competent agent that has not read the diff: the file,
  the cause, and what *done* would look like. The human's own words come with
  it, so it never has to guess what they meant by yes.
- **`fix` names the Option that means fix it** — `Q1.1` for a Question's,
  `Q1a.1` for a Sub-question's. It has to be an Option your Set actually offers,
  or the Set is refused: nothing else turns a finding into work.
- **Every finding in the block is a Question in the Set**, and the block goes on
  one Set and no others. This is the review, not a running commentary.
- **Read `verkstead guide` before you write it** — how a Set is labelled, how
  much belongs in one, and the shape it goes over the wire in. It ships inside
  the binary, so nothing else has to be found. A review that has found more than
  a sitting's worth of decisions is a review that should raise the ones that
  matter.

Then put it through `verkstead ask` and let it block. **The answers are not
yours to wait for**: Verkstead takes the Set from here, dispatches a session for
each finding they accept, and ends this one. You will not see the Response, and
there is nothing you would do with it.

If the ask itself fails — the server unreachable, any non-zero exit that is not
a refused Set — say so and stop. Never decide on their behalf.

## 5. A review that finds nothing

Ask nothing. **Say plainly, as the last thing you print, that you reviewed the
branch and found nothing worth raising** — that line is what the human sees on
the Timeline — and stop.

A Set with no findings in it is a row for them to dismiss, and the point of this
phase is to spend their attention only where there is a decision. Finding
nothing is a fine outcome; inventing a finding so that something happened is
not.
