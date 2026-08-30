# Conflict auto-resolution

A wrapping Conversation's pull requests are now watched for merge conflicts the
way their checks already are: the checks watcher's poll also reads GitHub's
`mergeable`, being conflict-free becomes a settle fact that gates Done, and a
CONFLICTING pull request gets a fix session dispatched at it — merge by
default, rebase where configured, two goes per PR and then a stop with a
Notice. After Done, a slow sweep keeps the fact fresh until each PR merges or
closes, the PR card draws a conflict indicator off it, and a resolve button on
a conflicted Done PR's details pane re-enters Wrapping with the review's settle
left standing so no review re-runs.

Settled by the grilling: watch during Wrapping (with a settle fact per PR so a
conflicted PR never sails to Done); companions included; silent dispatch,
Notice only on a stop; detection rides the checks watcher's existing `gh` call;
merge is the default strategy with a global setting and per-Repo override; the
Done side is a 15-minute sweep ending per PR at merged-or-closed; the indicator
draws wherever the last look said CONFLICTING; the button is a press of its
own, not a steer, offered only while conflicted.

## Tasks

- [x] 01: Detect conflicts and gate Done on them — [details](01-detect-and-gate.md)
- [ ] 02: Dispatch the resolution session — [details](02-resolution-session.md)
- [ ] 03: The resolution-strategy setting — [details](03-strategy-setting.md)
- [ ] 04: Watch Done Conversations' pull requests — [details](04-done-sweep.md)
- [ ] 05: Draw the conflict indicator — [details](05-indicator.md)
- [ ] 06: The resolve button — [details](06-resolve-button.md)
