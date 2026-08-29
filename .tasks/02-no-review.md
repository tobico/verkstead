# 02. "No review"

## What to build

**No review** joins the Review picker's flat list as one more row. It is a
real choice, stored apart from unchosen: an empty picker still refuses to
start, and picking "No review" satisfies readiness the way a Pairing does. It
freezes at grill start, is remembered per Repo like a pair, and a stage
inherits it like any review pick.

A Conversation carrying it wraps up without a review session. The wrap-up's
other watchers run exactly as today — the checks with their two fix attempts
per check, pull request comments with responding sessions, the settling loop —
so the wrap-up narrows straight to waiting-on-checks once nothing said on the
pull request is left unaddressed, and goes Done when the checks pass. With no
review there is nothing to split findings out of, so the
back-to-Implementing path simply never arises from a wrap-up like this.

## Acceptance criteria

- [x] "No review" satisfies readiness, freezes at start, is remembered per
      Repo and inherited by stages; an unchosen review picker still refuses
      the start.
- [x] A wrap-up on such a Conversation dispatches no review session, narrows
      to waiting-on-checks once comments are addressed, and carries the work
      to Done when the checks are green.
- [x] Check fixes and comment responses run exactly as they do today.
