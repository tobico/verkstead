# 01. Review on the picker, and a pull request taken up at Start

## What to build

**Review becomes a Process a human can pick, and Start on one takes up the pull
request the Brief names.** Both halves in one slice: a row offered over a press
that grills would start the wrong thing, and a take-up nothing can reach is not a
start.

**On the composer.** The Process picker offers *Review* and the server accepts
it — both lists grow a row, the viewer's and the record's. The per-Process role
table already says Review is run under the Implementation and Review roles with
the **Agent** control taking the panel shape, so what changes there is one thing:
the *No review* row goes for this Process, a Review without a review being Fix
Merge Issues with the comments answered. Develop and Tinker keep theirs.

**Readiness asks for a brief and both Pairings**, which is the reading a
wrap-up has always taken, and the inert Start says it is waiting on *both roles*
— counted off the role table rather than written into the sentence.

**Start presses the take-up rather than the start.** A Review's press goes to the
endpoint the *Wrap up* press went to, so every refusal it already has keeps its
name and its sentence; the button reads *Start work* like every other Process's,
there being one press on a composer.

**The target is resolved server-side, at the press.** The Brief is read for the
first `github.com/<owner>/<repo>/pull/<n>` URL or bare `#<n>` — the first of
either, wherever in the prose it falls — and that number is asked of the
configured `gh` in the Conversation's Repo, which resolves it against that Repo's
origin. What comes back is the pull request's title, its URL, its head branch,
the branch it merges into and whether its head is in a fork. A URL naming another
repository is refused by name: the ask answers for origin, so a URL whose owner
and repository are not the ones `gh` answered about is a target this Conversation
cannot take up.

No agent session does any of this. The take-up's refusals are careful and already
written, and a session doing the checkout would move them into a prompt.

**Then today's take-up, unchanged.** Fetch; the head branch settled against
origin — cut tracking origin's where there is none here, fast-forwarded where it
is behind, refused by name where it is ahead, has gone its own way, or is checked
out somewhere else, naming the place; the companions beside it; the head at
take-up as the base commit with GitHub's base branch beside it; and the pull
request recorded, which is the move from Draft into Wrapping and the start of the
ordinary wrap-up with its review, as found.

**And the record of what was taken up is written by this start.** The row that
used to be written when a pull request was loaded off the retired menu is written
here instead, out of what `gh` answered. It is not only bookkeeping: that row is
what lets a Draft through the one door into Wrapping, and it is what a
Conversation's Process is read back as for the whole of its life.

**Every refusal lands on the composer**, where the press was: nothing named in
the Brief, a URL naming another repository, a number GitHub has nothing open
under, a fork, a head that is ahead, diverged or checked out elsewhere, and one
another Conversation already holds — that last naming the Conversation and
leading there, since the branch is that Conversation's and there is one
Conversation per piece of work.

## Acceptance criteria

- [ ] A Draft can be set to Review, and its **Agent** panel draws Implementation
      and Review pickers with no *No review* row; Develop and Tinker still offer
      theirs.
- [ ] Start is inert while the Brief is empty or either Pairing is unanswered,
      saying it waits on a brief and both roles.
- [ ] Pressing a ready Review whose Brief names an open pull request — by URL or
      by `#n` — leaves the Conversation Wrapping on that pull request's head
      branch, with the worktree and every companion checked out, the pull request
      recorded, the head at take-up as the base commit and GitHub's base branch
      beside it, and the wrap-up's watchers running.
- [ ] A Brief naming nothing, a URL in another repository, a number nothing is
      open under, a fork, and a head that is ahead, diverged or checked out
      elsewhere are each refused by their own name on the composer.
- [ ] A pull request another Conversation holds is refused naming that
      Conversation, and the refusal leads there.
