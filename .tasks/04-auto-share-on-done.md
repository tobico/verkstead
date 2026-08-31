# 04. Auto-share when the wrap-up settles to Done

## What to build

One new recorded fact: **a share-to-PR comment was left on this Conversation**
— written whenever the comment lands, the pane's press and the auto-share
alike, and never unwritten.

When the wrap-up settles a Conversation to Done with `share_on_done` on and no
such fact on record, Verkstead publishes a fresh share and comments the link on
every pull request, exactly as the pane's press does — this is the pipeline's
own settle, in the same breath as the push and the unseen mark. A steer into
Done runs nothing and shares nothing.

The gate is the fact, not the arrival: a hand-shared Conversation is already
commented and stays quiet, a later settle after a successful share stays quiet,
and a settle whose share **failed** left no fact, so the next settle tries
again.

Failure — no token, a token without the gist scope, a refused publish, or a
pull request the comment could not land on — is a Notice on the Timeline naming
what failed. Success writes nothing to the Timeline; the fresh publish reads in
the Share pane.

## Acceptance criteria

- [ ] A wrap-up settling to Done with the toggle on leaves the comment on
      every pull request; with it off, none.
- [ ] A Conversation with a comment already on record — from the pane or an
      earlier settle — is left quiet, and so is every steer into Done.
- [ ] A failed publish or comment writes a Notice naming the failure, and the
      next settle to Done tries again.
- [ ] Success writes nothing to the Timeline, and the pane shows the fresh
      publish.
