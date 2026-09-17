# 06. Done checks for the pull request

## What to build

Every run here ends on a pull request, and each session that ends one commits its
work and then pushes and opens the pull request *after* the commit. So each can
land everything it was sent for and stop short of the one act that makes the work
reviewable. Today that is caught afterwards: GitHub is asked once the session is
over, and a second session is sent for nothing but the push and the pull request
(`to_a_pull_request`, the submitting skill). With a done signal it can be caught
in the same turn, which is cheaper than a second session. See
`docs/adr/0018-a-session-says-it-is-done.md`.

**For a session meant to end on a pull request, the signal is refused while the
branch has none open.** Those sessions are: a backlog's **finish step** (not its
task steps), an **inline implementation**, a **roadmap's own session** (the
staging tail of a grilling, and the staging a Resume launches), and the session
**sent to open the pull request**. The refusal says the branch has no open pull
request and to push and open one the way the repository's own review process
says. Ask GitHub the way the runner already asks after these sessions, from the
right directory, and mind that these may be stacked pull requests — it is this
branch's own pull request that is asked about.

**GitHub out of reach reads as accepted.** No `gh`, no authentication, no
network, a rate limit, an error that is not an answer: the signal is accepted,
with a log line saying the pull request could not be checked. A session is not to
be held hostage by somebody else's outage, and what comes after it still asks.

**The second session stays**, as the net under a session that exits by itself
without signalling, or one accepted because GitHub could not answer. Nothing
about when it is sent changes; it should simply become rare.

Add the pull request to what the Guide's section on ending a session says is
checked, and make sure the `next-task` (finish step), `implementing`, `staging`
and `submitting` skills put the push and the pull request before `verkstead
done`.

## Acceptance criteria

- [ ] A finish step, an inline run, a roadmap session and an open-the-pull-request session are each refused while their branch has no open pull request, and accepted once it has one
- [ ] A backlog task step, a breakdown and a handoff are not asked about a pull request
- [ ] With GitHub unreachable the signal is accepted and a log line says the check could not be made
- [ ] A session that exits without signalling and without a pull request still gets the second session, as before
- [ ] The Unix and Windows session suites pass
