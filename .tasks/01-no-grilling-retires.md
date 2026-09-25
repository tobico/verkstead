# 01. No grilling retires

## What to build

The Grilling picker stops offering a row that is not an account. Every
Conversation Develop starts is grilled, and the Brief that wants no interview is
a Tinker — which the stage's later tasks build. So the row goes, and with it
everything behind it: the picker's own `NONE` handling for that role on both
composers, the store call that records the skip, the second landing the start
press had, and the prompt an ungrilled session was started on.

Four things behind the row, and each is a different place:

- **The picker.** The per-Process table already says which roles may be picked
  away; the grilling role comes off Develop's list, and the words that named the
  row go with it. The Review role's row is untouched — Develop and Tinker both
  offer it.
- **The start.** The press has one landing again. The store's start took an
  optional direction that said both *which state to land in* and *record an
  inline direction*; a Conversation that skipped the interview was the only
  caller of the second, and the state it landed in was the only reason the
  parameter was optional. What is left is a start that lands Grilling.
- **The session.** The inline build launched straight off the press, and the
  prompt that told it there was no interview and the Brief was the whole plan,
  both go. The implementation skill's own paragraph saying a Conversation can be
  started with no grilling at all goes with them.
- **The memory.** `repo_skips` still holds a grilling skip against Repos that
  were last started with one. The read tolerates it and does not apply it — the
  picker prefills empty, exactly as it does for a remembered Pairing whose
  Profile has broken — and nothing ever writes one again. A roadmap stage stops
  inheriting one from its predecessor too: it inherits a grilling Pairing or
  nothing.

Readiness keeps its shape and loses one case: the grilling role is answered with
a Pairing or it is not answered, so an empty picker is what Start waits on and
there is no third state. The button still reads **Start work**.

The docs that still describe the row are corrected with it — the adoption guide's
account of the three pickers and of what the press does. The design document and
`CONTEXT.md` already say the row is retired, so they need nothing.

## Acceptance criteria

- [ ] Neither composer offers a row for the grilling role that is not an
      account, and a Draft whose Grilling picker is empty is refused at Start
      with the refusal that names the missing grilling Profile.
- [ ] A Repo whose remembered set holds a grilling skip gives a new Conversation
      an empty Grilling picker, and starting that Conversation writes no skip
      back for the role.
- [ ] Nothing in the workspace records or reads a grilling skip on a
      Conversation, lands a Draft straight in Implementing, or builds the
      *Nothing was grilled* prompt, and the tests that covered those paths are
      gone rather than disabled.
- [ ] A roadmap stage started from a predecessor inherits that predecessor's
      grilling Pairing, and a predecessor with none leaves the stage's picker
      empty.
