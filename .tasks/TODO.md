# Propose-then-fix wrap-up

The wrap-up stops splitting every fix into a fresh session of its own, and
stops acting on pull request comments ungated. One wrap-up session reviews the
branch and the pull request's comments together, proposes every fix in one
Question Set, and on approval fixes and pushes in the same session — offering
to split findings out into a backlog only when it judges the work too big for
one sitting. Red checks that go red mid-wait fold into the woken session, and
approved fixes that fail to land stop the run at an Interruption rather than
disappearing.

## Tasks

- [x] 01: The wrap-up session fixes what was approved, itself — [details](01-session-fixes-inline.md)
- [x] 02: Approved fixes cannot be silently dropped — [details](02-no-dropped-fixes.md)
- [x] 03: The review reads the pull request's comments — [details](03-comments-join-review.md)
- [ ] 04: Comment batches propose before they fix — [details](04-responding-skill.md)
- [ ] 05: Red checks fold into the woken session — [details](05-checks-fold-in.md)
- [ ] 06: A finding can offer a split Option — [details](06-split-option-schema.md)
- [ ] 07: A Conversation can wrap up twice — [details](07-second-wrap.md)
- [ ] 08: A split pick becomes a backlog — [details](08-split-becomes-backlog.md)
