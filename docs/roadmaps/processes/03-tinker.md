# 03. Tinker, and No grilling retires

## Goal

A draft whose Process is **Tinker** starts straight into Follow-up: Start
fetches, resolves the base, cuts the branch and the Worktree and every
companion's, freezes the Brief and the Pairings, and launches one session under
the Implementation Pairing on the `following-up` skill, primed with the Brief
as the thing to follow up on and told to rename the branch. The human answers
its rounds and ticks **Nothing else** as they do for any follow-up; when the
session then signals, a branch holding commits carries the Conversation into
Wrapping — the `submitting` step opens the pull request and the ordinary wrap-up
runs, reviewed where a Review Pairing was picked — and a branch holding none
carries it to Done. The *No grilling* row is gone from the Grilling picker, the
ungrilled start path and the *Nothing was grilled* prompt are gone with it, and
a skip a Repo remembers for the role is not applied.

## Decisions in force

- **[ADR-0020](../../adr/0020-a-conversation-has-a-process.md), *Tinker***:
  Follow-up entered from a Draft, rounds for as long as the human wants, and
  the ending rule that reads the branch. It is what replaces *No grilling*; a
  sixth Process holding the ungrilled inline build under a name was considered
  and rejected, so nothing of that path is kept.
- **The start is the grill start's own sequence** with a different landing
  state and a different first session — the one shape `start_grilling` and the
  retired `start_building` shared, now shared by Tinker. The naming instruction
  is appended as for every first session.
- **The session is the follow-up's session.** Same skill, same prompt builder
  with the Brief in the *What I want to follow up on* place and no handoff, same
  Nothing-else mark on its Sets, same Done-signal evidence — the mark — and the
  same refusal without it. What differs is that the branch is on no pull
  request yet, so the skill's talk of pushing and checks is conditional on one:
  the skill text says what to do when the branch has no pull request, which is
  commit and carry on, and the session is not told to open one.
- **The ending reads the branch** in the place `follow_up_over` already
  computes whether anything was pushed: commits since the base and no pull
  request means Wrapping, where the wrap-up's existing *no pull request* entry
  dispatches `submitting`; no commits means Done, with a Moved line and nothing
  dispatched. A follow-up steered into on a pull request is unchanged.
- **Roles are Implementation and Review, Review skippable.** Readiness asks for
  those two; the Grilling picker is not drawn for Tinker at all.
- **No grilling retires** (CONTEXT.md, **Pairing**): the row, the compose
  page's `NONE` handling for the grilling role, the `skip_grilling` path for
  that role, `start_building`, the ungrilled prompt and their tests. A
  remembered grilling skip in `repo_skips` is not applied, exactly as a broken
  remembered Pairing is not — read as nothing chosen — and never written again.
  `Picked::Skipped` stays for the Review role.

## Proposed tasks (provisional)

1. **The Tinker start** — Start on a Tinker draft cuts and freezes as a grill
   start does and lands Follow-up, launching the follow-up session primed with
   the Brief. AC: the Conversation reads Follow-up with a worktree and a
   branch; the session's prompt carries the Brief and the naming block; the
   Timeline shows the move.
2. **The follow-up skill on a bare branch** — the skill says what to do with no
   pull request yet, and the Nothing-else box is drawn on its Sets. AC: a round
   without a pull request commits and asks; the box is on the Set.
3. **The ending** — mark plus signal on a Tinker reads the branch: commits mean
   Wrapping with `submitting` dispatched, none mean Done. AC: a Tinker that
   committed lands in Wrapping and a pull request opens; one that did not lands
   Done with nothing running; a steered follow-up on a pull request behaves as
   before.
4. **Tinker on the picker** — the Process row appears, readiness asks for two
   roles, the Agent control takes the panel shape with Review skippable. AC: a
   Tinker draft's Start waits on a brief and two roles; the Grilling picker is
   absent.
5. **No grilling goes** — the row, the start path, the prompt, the memory
   read, the tests; the design doc and CONTEXT.md already say so. AC: no
   *No grilling* row anywhere; a Repo remembering a grilling skip prefills the
   grilling picker empty; `start_building` no longer exists.

## Re-verify at start

- Stages 01 and 02 landed: the Process is on the record and the Agent control
  reads the role table.
- `follow_up_over` in the store is still where the follow-up's ending is
  computed, with the pushed flag beside it.
- The wrap-up still dispatches `submitting` on entry where the branch has no
  pull request, which is what Tinker's ending relies on.
- The `following-up` skill still ends its rounds with one ordinary Set and
  still says nothing about a branch with no pull request.
- `repo_skips` still holds a remembered grilling skip for some Repos, so the
  read has to tolerate one.
