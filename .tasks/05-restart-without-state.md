# 05. Restart without state

## What to build

A server restarted mid-rounds, and a Resume pressed on a Follow-up, start the
follow-up rounds component again with the same start facts and the Timeline as
it now stands. The component derives where it was from those and from git
(Set 1767, Q31). Nothing new is stored: no table, and no per-Activity key-value
store.

Today `resume.rs` and the startup resume reach `runner::follow_up_again`, which:
- locks the question a dead session left (`FollowUp::again`);
- primes the fresh session with the rounds already answered;
- asks for a pull request instead of another round where a Tinker's ending got
  as far as owing one (`owed_a_pull_request`: a marked round, no pull request,
  commits standing).

All of that is read off the Timeline and git, so it moves into the component's
start. The relaunch sees a marked round with work standing and ends `committed`
at once, and the host's mapping sends the `submitting` session.

## Acceptance criteria

- [ ] `a_restart_follows_the_work_up_again_rather_than_raising_anything` and `resume_follows_the_work_up_again_on_the_brief_and_the_rounds_answered` pass unchanged under the component.
- [ ] `resume_tinkers_again_on_the_brief_and_the_rounds_answered` and `resume_on_a_tinker_that_owes_a_pull_request_sends_for_it_rather_than_another_round` pass unchanged.
- [ ] No migration or table is added for the Activity's state.
