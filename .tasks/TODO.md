# Wrap-up

Takes a Conversation from implemented work to a reviewed, settled pull request,
and makes a staged roadmap execute itself. The finish sequence stops waiting for
anybody: a backlog worked to empty pushes and opens its own draft PR, and the
Conversation moves into Wrapping. There Verkstead watches the PR through the
host's `gh` — its checks, its comments — and a fresh-context session reviews it
and raises what it finds as a Question Set. Failing checks and unaddressed
comments dispatch fix sessions on their own; two failed attempts at the same
check is where it stops and asks.

Wrap-up settling is what lets the next roadmap stage start, so the last of it is
the roadmap direction becoming executable: a bundled fork writes
`docs/roadmaps/`, the stage list is pinned beside the task list, and each stage
opens its own Conversation on a branch stacked on the one before it. When this
lands, Verkstead replaces roadrunner and the tobico-scripts wrappers for daily
work.

Roadmap stage: [04: Wrap-up](docs/roadmaps/mvp/04-wrap-up.md)

## Tasks

- [x] 01: Host gh and the PR Event — [details](01-host-gh-and-the-pr-event.md)
- [x] 02: CI monitoring and fix sessions — [details](02-ci-monitoring.md)
- [ ] 03: The wrap-up self-review — [details](03-self-review.md)
- [ ] 04: PR comments and settling — [details](04-comments-and-settling.md)
- [ ] 05: Roadmap direction and the stage list — [details](05-roadmap-and-stage-list.md)
- [ ] 06: Stage auto-continue — [details](06-stage-auto-continue.md)
- [ ] 07: Retirement pass — [details](07-retirement-pass.md)
