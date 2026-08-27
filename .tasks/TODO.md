# Follow-up conversations

A new **Follow-up** state, reached only by a steer once a Conversation's work
is on a pull request. The human writes a brief; a session runs rounds of
ordinary Question Sets — answering questions, doing follow-up work, pushing
each round — until the human picks a **Nothing else** option the UI draws on
the follow-up's Sets, at which point the Conversation lands back in the
wrap-up and settles to Done the ordinary way. Around it: the wrap-up's
checks-only wait becomes a visible **Waiting on checks** condition, and a
**rescue** — a canned line typed into a session that has gone idle without
asking or finishing — is added for follow-ups and then generalized to every
session.

## Tasks

- [ ] 01: Draw the narrowed wrap-up as Waiting on checks — [details](01-waiting-on-checks.md)
- [ ] 02: Ship the following-up skill and its prompt — [details](02-following-up-skill.md)
- [ ] 03: The fifth steer target, the state, and the session it launches — [details](03-steer-into-follow-up.md)
- [ ] 04: The Nothing-else control on a follow-up's Sets — [details](04-nothing-else-control.md)
- [ ] 05: End the follow-up on the mark, and land in the wrap-up — [details](05-follow-up-done-rule.md)
- [ ] 06: The rescue, and Resume, for Follow-up — [details](06-follow-up-rescue-and-resume.md)
- [ ] 07: Extend the rescue to every session — [details](07-rescue-everywhere.md)
- [ ] 08: The vocabulary — [details](08-vocabulary.md)
