# 03. "No grilling"

## What to build

**No grilling** joins the Grilling picker's flat list as one more row. With it
picked, readiness asks for no Grilling Pairing — the Implementation and Review
picks and a non-empty Brief are still required — and the start press does
everything a grill start does (fetch, resolve the base, cut the branch, make
the Worktree and every companion's, freeze the Brief and the setup) but moves
the Conversation straight to Implementing.

What launches is a fresh session under the Implementation Pairing on the
implementing skill, primed with the Brief alone and told in the prompt that
nothing was grilled: the Brief is the whole plan, and a real decision it
leaves open is the session's to put as a blocking ask rather than to guess.
This start is inline only — no backlog, no roadmap — and the run is watched
out to a pull request and an ordinary wrap-up exactly as an inline
implementation is.

"No grilling" is remembered per Repo the way a pair is. The start button
reads one neutral label, **Start work**, whichever way the Conversation
starts.

## Acceptance criteria

- [ ] With "No grilling" picked and the other picks made, the press lands the
      Conversation Implementing with a session primed on the Brief alone;
      no Grilling Pairing is required and blocking asks work from it.
- [ ] The session's work is carried to a pull request and an ordinary
      wrap-up, like any inline implementation.
- [ ] "No grilling" is remembered per Repo, and the start button reads
      "Start work" in both modes.
