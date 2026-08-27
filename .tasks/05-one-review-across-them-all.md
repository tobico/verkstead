# 05. One review across every pull request

## What to build

One review session reads the whole of the work, across every pull request it
ended up on, and puts what it finds as one Question Set.

**One review, not one per pull request.** The work was conceived as one
Conversation and reads best whole: a change in a companion and the change in
the Conversation's own repository that needed it are one thing to judge, and
two sessions each seeing half would each be the frame this phase exists to
escape. So the review settles once, as it does now, and the comments gate still
waits on that one review.

**The prompt lists every pull request.** The review session is told each one —
its repository, its number, its URL and the worktree to run `gh` in — because a
session runs in the Conversation's own worktree and `gh` reads its repository
from where it runs. A Conversation whose work touched nothing else is told what
it is told today.

**The reviewing skill is written for several.** It is written for one branch
and one pull request throughout: it reads one diff, it asks one pull request
how its checks are getting on once the answers arrive, it pushes once at the
end, and it says not to touch any other branch. Each of those becomes per
repository — a diff read in each worktree, each pull request's checks asked
about where that pull request lives, a push from every worktree it committed
in, and a rule about other branches that still holds when a session may
legitimately commit in more than one place.

**The findings are still one Set.** A finding names the repository it is about
where that is not the Conversation's own, so an Option the human picks says
plainly what would change and where. Everything else about the phase is
unchanged: it proposes, they answer, it fixes what they accepted, and a finding
they declined is never raised again.

**And the split-out path still works.** A review that judges a finding too big
for the sitting writes a `.tasks/` backlog instead, the Conversation goes back
down the ladder to build it, and the finish that follows wraps it up again on
the pull requests it already had — reviewed afresh, and now there may be
several of them.

## Acceptance criteria

- [ ] The review session is told every pull request the Conversation holds —
      repository, number, URL and the worktree to run `gh` in — and the skill
      says to read each one's diff where that pull request lives.
- [ ] A review that accepts findings in two repositories commits in both
      worktrees and pushes both, and the checks it folds in are each asked of
      the right pull request.
- [ ] The review still settles once, the comments gate still waits on that one
      review, and a Conversation with a single pull request gets the session it
      gets today.
- [ ] A review that splits its findings out into a backlog still takes the
      Conversation back to Implementing, and the second wrap reviews every pull
      request afresh.
