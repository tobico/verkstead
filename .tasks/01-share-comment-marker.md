# 01. The share comment stops triggering Wrapping

## What to build

Share to Pull Request currently leaves a comment that Wrapping's own comment
watcher picks straight back up and dispatches a session to address. The comment
is posted by the configured GitHub token — usually the human's own account — so
no author rule could tell it from comments the human writes themselves.
Something in the body has to identify it instead.

Give the itemized share comment an invisible marker: an HTML comment on its own
line at the end of the body, invisible in GitHub's rendering. Then teach the
comment reading that Wrapping runs on to drop any comment in which a line
*starts* with the marker — built in, never configurable, and applied everywhere
Wrapping reads comments: the fresh comments that would dispatch a batch
session, and the pre-existing comments folded into the review prompt. Companion
repositories' pull requests go through the same reading and are covered by the
same drop.

The line-start rule is deliberate: a human quote-replying to the share comment
gets every quoted line prefixed with `>`, so their reply does not carry the
marker at a line start and is still addressed. The marker does not need
recording as addressed — it is dropped by inspection on every poll, for ever.

Nothing else about the share changes: the body's visible content, the posting
account, and the one-comment-per-pull-request behaviour all stand as they are.

## Acceptance criteria

- [ ] The posted share comment's body ends with the marker on its own line, and nothing visible changes in GitHub's rendering of it.
- [ ] A comment carrying the marker at the start of a line is never dispatched for and never folded into the review prompt — sharing to a watched pull request spins up no session.
- [ ] A quote-reply of the share comment (each line `>`-prefixed) is still treated as an ordinary comment to address.
- [ ] Existing share-comment body assertions and wrap-up comment tests are extended rather than weakened.
