# 01. The Review Pairing, end to end

## What to build

A third Pairing role, **Review**, beside Grilling and Implementation. It is a
role a Pairing is used in, exactly as the existing two are: the same flat
Profile-and-model list, picked on the setup card while the Brief drafts, frozen
at grill start with the others, and read on the details pane after.

The role reaches exactly one kind of session: the wrap-up's review — including
the fresh review that runs after a split-out backlog is built. Every other
session the wrap-up dispatches (check fixes, comment responses, follow-ups,
the session sent for a missing pull request) stays under the Implementation
Pairing, the line being that reviewing is a fresh set of eyes and fixing is
building.

The pick is required: a draft is not ready to start without it, the same rule
the other two follow. Each Repo remembers the last review pick the way it
remembers the pair, prefilling the next draft and silently not applying a
remembered pick whose Profile has broken or no longer lists the model. A
roadmap stage inherits its predecessor's review pick through the same act that
gives it the other two Pairings. Steers that settle a Pairing settle this one
the same way where the target runs a review.

## Acceptance criteria

- [ ] A draft refuses to start until a Review Pairing is picked; the pick
      freezes at grill start and is readable afterwards like the other two.
- [ ] The review session — and the re-review after a split-out backlog is
      built — runs under the Review Pairing, while check-fix, responding and
      follow-up sessions still run under the Implementation Pairing.
- [ ] The last review pick prefills the next draft on that Repo, a broken or
      stale remembered pick is silently not applied, and a stage inherits its
      predecessor's.
