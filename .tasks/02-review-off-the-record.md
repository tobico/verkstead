# 02. Key the review's outcomes on the branch and the session

## What to build

The server currently reads the review Set's `review.findings` block to know
what a dead session still owed — which picks meant *fix it* or *split it out* —
and dispatches fresh sessions briefed from the record. That whole safety net is
being removed by decision: the findings grammar is going (task 04), and with it
everything that reads the record to decide what a review left behind.

What replaces it keys on the branch and on the session:

- **A review session that ends cleanly settles the review** — unless a fresh
  `.tasks/` backlog is on the branch, in which case the Conversation goes back
  to Implementing to build it, exactly as a split pick moves it today. The
  backlog on the branch is the whole signal; no record is consulted.
- **A review session that dies — or a server restart or Resume that finds a
  review with no session behind it — is a stop**, whatever was or was not
  answered. Any unanswered Set the dead session left is closed, so no question
  stands on the Timeline with nobody behind it, and Resume runs a fresh review
  that reads the branch and asks again. Nothing is ever dispatched from the
  record: the owed-fixes paths, the unattended-answered dispatch and the
  feedback prompt built from stored findings all go.

The Conversation-level flow around it is unchanged: settling the review is
still one of the three things the wrap-up waits on, and the move back to
Implementing still carries the review's settle with it.

## Acceptance criteria

- [ ] A review session that ends cleanly with no fresh backlog settles the
      review without any findings record being read.
- [ ] One that ends cleanly having committed a fresh `.tasks/` backlog sends
      the Conversation back to Implementing, and the backlog is then worked as
      any other is.
- [ ] A review session that dies, and a restart or Resume over a review with no
      session, stop the run with the reason on the Timeline, close any
      unanswered Set the session left, and dispatch nothing; Resume runs the
      review over from the start.
- [ ] The server tests covering the owed-fixes and unattended paths are
      replaced by tests of the new endings.
