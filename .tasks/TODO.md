# inline-pr-and-fresh-base

Two fixes to the pipeline, settled with the human over a grilling. First: an
inline implementation never reaches a pull request — the implementing skill
forbids the session to push or open one, promising a step Verkstead runs after,
and that step was never built, so the Conversation sticks in Implementing and
even a hand-opened PR is never found. Second: nothing ever runs `git fetch`, so
a new Conversation's unpicked base resolves the *local* default branch and
comes off wherever it last stood, not origin's tip.

Note for test runs: the guide test fails when `VERKSTEAD_SERVER` is set in the
environment — run `env -u VERKSTEAD_SERVER cargo test` if it bites.

## Tasks

- [x] 01: An inline run ends on its pull request — [details](01-inline-run-ends-on-its-pull-request.md)
- [x] 02: Resume finds an inline run's pull request — [details](02-resume-finds-an-inline-runs-pull-request.md)
- [ ] 03: A new Conversation branches from origin's fresh default tip — [details](03-new-conversation-branches-from-origins-fresh-default-tip.md)
- [ ] 04: Stages and Adopt come off a fresh default too — [details](04-stages-and-adopt-come-off-a-fresh-default-too.md)
