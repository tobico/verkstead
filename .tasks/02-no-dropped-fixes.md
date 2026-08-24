# 02. Approved fixes cannot be silently dropped

## What to build

A wrap-up session that ends without landing the fixes the human accepted stops
the run at an Interruption instead of letting the Review settle. The failure
this closes: the human answers the review's Set, the session dies before its
push, and wrap-up reaches Done with approved fixes quietly gone.

Detection is mechanical: the review's Set was answered with at least one
accepted finding, and the session ended without landing them — no commit on
the branch since the answers came back. The Interruption says what is
unlanded, in the words the review wrote.

The retry dispatches **one** fix-only session inside the addressing skill,
handed every accepted finding together with whatever the human wrote beside
each answer, exactly as the review would have told a fix session before this
rework. Nothing is re-asked: the decisions were made, and only the doing
failed.

## Acceptance criteria

- [ ] A wrap-up session that ends after the answers arrive but before pushing
      raises an Interruption on the Timeline, and the Review does not settle.
- [ ] Retrying that Interruption dispatches a single addressing session whose
      feedback carries all the accepted findings and the human's words, and no
      Question Set is asked again.
- [ ] A session that ends cleanly having pushed its accepted fixes raises
      nothing, and one whose review had nothing accepted settles as an ordinary
      clean end.
