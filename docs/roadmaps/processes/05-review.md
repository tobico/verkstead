# 05. Review

## Goal

A draft whose Process is **Review** takes a pull request or a branch and wraps
it up. The Brief is scanned for the first pull request URL or `#number`; the
Repo panel's Branch field reads *Pull request or branch* for this Process,
accepts any of the three, and fills from the Brief while empty; Start is
refused, saying so on the composer, while neither names anything. Start runs
today's take-up — fetch, the head cut or fast-forwarded and a head that is
ahead, diverged, checked out elsewhere or from a fork refused by name, the head
at take-up as the base commit and GitHub's base beside it — and records the
pull request, which moves the Conversation into Wrapping for the ordinary
wrap-up with its review. A bare branch is taken up the same way with no pull
request recorded and the base picker's base kept; the wrap-up's `submitting`
step opens the pull request against it first. *Wrap up a pull request* and the
open-pull-request list behind it are gone from Other actions, which keeps
*Continue a roadmap* as its one level. Two roles, Implementation and Review,
with no *No review* row.

## Decisions in force

- **[ADR-0020](../../adr/0020-a-conversation-has-a-process.md), *Review*
  and *Naming a pull request or a branch***: what the Process is, where the
  target is named and why a bare branch goes in the field, what Start refuses,
  and what retires. The take-up rules themselves are unchanged from
  CONTEXT.md's **Adopt** entry, now run at Start rather than from a press on
  the draft's pane.
- **Server-side parsing at Start**, not an agent session: the take-up's
  refusals are careful and already written, and a session doing the checkout
  would move them into a prompt. The Brief is read for the first
  `github.com/<owner>/<repo>/pull/<n>` URL or bare `#<n>`, resolved through
  the configured `gh` against the Repo's origin; a URL naming another
  repository is refused by name.
- **The field is the Branch field with a different reading**, not a new
  control: for Review the label reads *Pull request or branch*, the automatic
  placeholder is gone, and the value is a pull request or a branch. Which of
  the two it holds is decided at Start: a URL or `#n` is a pull request, and
  anything else is a branch that must exist on origin. The Brief's URL fills
  the field while empty and never overwrites what the human typed.
- **A bare branch is a take-up with no pull request**: the same fetch and the
  same refusals, the head at take-up as the base commit, the base picker's
  base as `base_ref`, and Wrapping entered with no pull request recorded — the
  wrap-up's existing entry for a branch without one dispatches `submitting`,
  which opens it as a draft. The Timeline draws only what Verkstead adds.
- **The Brief is the human's** and no longer the pull request's title and
  body. Nothing about the pull request is touched.
- **Two roles, Review required** — no *No review* row for this Process, a
  Review without a review being Fix Merge Issues. Readiness asks for a brief
  and both Pairings; the Agent control is the panel.
- **Retirements**: the *Wrap up a pull request* level, the
  `open-pull-requests` endpoint and the `gh` read behind it, the
  pull-request-adoption start endpoint and the *Wrap up* press on the draft's
  pane, and their tests. `pull_request_adoptions` stays as the record of what
  was taken up, written by the new start; a Draft from before holding one
  reads as Review and its Start is the new one.
- **Refusals land on the composer** where the press was, as a refused take-up
  does today: no target, a target another Conversation holds (leading there),
  a fork, a head that is ahead or diverged, a branch not on origin.

## Proposed tasks (provisional)

1. **Naming the target** — the Brief scanned for the first URL or `#n`; the
   Branch field re-labelled and re-read for Review, filled from the Brief while
   empty; Start refused on the composer while nothing is named. AC: a Brief
   with a URL fills the field; a typed branch survives a URL arriving in the
   Brief; Start on an empty pair is inert and says why.
2. **Take-up at Start** — the Review start resolves a pull request through
   `gh`, runs the existing take-up and records it, landing Wrapping; every
   existing refusal keeps its name. AC: a Review draft naming an open pull
   request lands Wrapping on its head with the pull request recorded; a
   diverged head is refused by name; one another Conversation holds leads
   there.
3. **A bare branch** — the same start over a branch on origin with no pull
   request, base from the picker, and the wrap-up opening the pull request.
   AC: a Review of a branch lands Wrapping and `submitting` opens a draft pull
   request against the picked base; a branch not on origin is refused.
4. **Retirements** — the menu level, the list endpoint, the old start and the
   press, the pull request's words as a Brief; Other actions keeps one level.
   AC: Other actions offers *Continue a roadmap* alone; no request for open
   pull requests leaves the page; a Draft from before that adopted a pull
   request starts through the new path.
5. **Review on the picker** — the row, the panel with no *No review*,
   readiness on two roles, the base picker hidden for a pull request and kept
   for a branch. AC: a Review draft's Start waits on a brief, a target and two
   roles.

## Re-verify at start

- Stages 01 and 02 landed; the Branch field is still the shared setup row's
  and still reads the automatic placeholder for a Develop draft.
- The take-up in `crates/server/src/conversations.rs` still runs fetch,
  `settled`, companions, `store::take_up` and `record_pull_request` in that
  order with the refusals named there.
- The wrap-up's entry still dispatches `submitting` where the head has no
  pull request, and `submitting` still opens a draft.
- `gh` is still the configured way to GitHub and still answers a pull request
  by number for a Repo's origin.
- The design doc's Other actions paragraph is still the record of why one
  menu holds every action, so one level left is still drawn as that menu.
