# 02. CI monitoring and fix sessions

## What to build

A Conversation in Wrapping watches its pull request's checks, and fixes them
itself when it can.

Verkstead polls the PR's checks through the host `gh` for as long as the
Conversation is Wrapping, and stops when it leaves. Checks still running are
nothing to do. Checks that pass settle one of the three things wrap-up is
waiting on — see task 04, where the settling rule lands.

A check that fails dispatches a fix session: a fresh session under the
Conversation's implementation Profile, inside the bundled **addressing** skill.
That skill is new here and is written once for three callers — a failed check,
a review finding, an unaddressed PR comment. All three hand an agent a piece of
feedback and expect a commit, and one skill saying how to take feedback and
land a fix is worth more than three that drift apart. What differs between them
is the feedback in the prompt, not the instructions above it. It commits as
every other session does, with no gate, and the branch watcher from stage 03
puts what it committed on the Timeline.

**Two attempts, then it stops asking the machine and starts asking the human.**
A fix session that ends with the checks still failing gets one more. The second
failure raises an Interruption — the run does not go round a third time. That
is a blocking ask in the ordinary sense the rest of Verkstead means it: the
Conversation carries *blocked on you*, and the evidence is what makes the choice
answerable without opening a terminal — which checks failed, what the fix
sessions did, and the tail of what the last one said. The count is per check
rather than per Conversation: a suite where one job fails and is fixed and then
a different one fails has not spent its attempts.

## Acceptance criteria

- [ ] Checks are polled through host `gh` while the Conversation is Wrapping,
      and polling stops when it leaves.
- [ ] A failing check dispatches exactly one fix session, under the
      implementation Profile, inside the bundled addressing skill.
- [ ] The addressing skill exists and is written to serve a failed check, a
      review finding and a PR comment alike.
- [ ] A second failure of the same check raises an Interruption carrying which
      checks failed and the last session's tail, and nothing further is
      dispatched for it.
- [ ] Checks passing dispatches nothing and records CI as settled for that PR.
- [ ] A `gh` that cannot answer about checks leaves the Conversation waiting
      rather than reading as either green or failed.
