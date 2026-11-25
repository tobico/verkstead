# 03. The job-must-not-change short-circuit

## What to build

The incident's fix. When a conversation is waiting to be carried on, the launch
must be the session that was cut off — not a chance for the launch to re-decide
the job. In the incident, the recompute after a stop decided "this follow-up is
over and its work is on no pull request" and sent a `submitting` session to open a
pull request, instead of letting the follow-up act on the answers the human had
just given.

The drivers that re-derive the job before launching a session —
`runner::follow_up_again` (its `owed_a_pull_request` diversion), the backlog's
`runner::backlog_again` where it decides a finish-vs-work branch, and any sibling
that makes such a call — consult the `store::carrying_a_conversation` peek at
entry. Where a conversation is being carried on, they launch the resumed session
in its **current role** and skip the job-diverting sub-decisions. The carried
session's own turn decides the job, not the recompute.

Use the existing peek (it reads the row without spending it, which is exactly what
a pre-launch check needs); the row is spent where it always is, in
`sessions::continuing`. A force-stopped cut-off (from task 01) is enough to drive
the test — the sign-out flavour of the full incident lands once task 04 adds
sign-out.

## Acceptance criteria

- [ ] A follow-up cut off mid-answer is carried on, not diverted into opening a pull request.
- [ ] No job-diverting sub-decision (`owed_a_pull_request`, the finish-vs-work branch) runs while a conversation is being carried on.
- [ ] The incident shape — a cut-off `submitting`/follow-up that must act on its answers rather than open a pull request — is reproduced as a test and passes.
