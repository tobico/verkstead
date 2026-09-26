# 03. The ending, over scratch and at Done

## What to build

**An Investigate Conversation ends when the human's mark and the session's Done
signal meet, and it ends Done.** The whole of the Investigate Process works after
this task; the steered case is tasks 04 and 05.

**A kind of its own on the Done signal's evidence table.** It is the follow-up's
mark — the newest round the human answered carries **Nothing else**, and a signal
without it is refused so that the next round goes to them as a Set — with two
differences the rules carry:

- **Uncommitted changes do not refuse it.** The scratch is the point: the Diff on
  every Set already shows it, and the Worktree goes with the close. This is the
  one place that check is skipped, so it stays a step of its own that this kind
  passes by rather than a check that softens for everybody.
- **The session ends with its work rather than on a pull request.** No pull
  request is ever asked for, here or in any companion.

The refusal for a missing mark is worded for an investigation rather than for a
follow-up, the two now sharing the mechanism and not the words.

**The mark is read inside this Investigating's own window.** The reading that
finds the newest answered round windows it by the last move *into* the state, and
it has to be told which state's window to look in rather than assuming the
follow-up's — a Conversation can be investigated more than once, and the round
before this one is finished with.

**The ending lands Done, and dispatches nothing.** No wrap-up, no watchers, no
`submitting` session, no pull request sent for: the move and the line on the
Timeline are the whole of it, and the Worktree stays as it does for any Done
Conversation. Nothing is pushed to the devices and nothing is stamped unseen
either — the human ended this themselves a moment ago by ticking the box. The
store refuses the move for anything but Investigating, as every move is refused
outside the state it leaves.

**And a session gone on a marked round is an ending rather than a stop**, read
the way a follow-up's is: where the session ends first, the record is asked once
more — no Set standing open, and the newest round marked — and an investigation
that reads as finished lands Done instead of stopping.

## Acceptance criteria

- [ ] A signal from an investigating session on a round the human has not marked
      is refused, in the investigation's own words, and the session carries on.
- [ ] With the newest answered round marked, the signal is accepted although the
      Worktree holds modified, staged and untracked files, and nothing asks for a
      pull request.
- [ ] An Investigate Conversation that started from its own Draft reads **Done**
      afterwards, with the move on the Timeline, nothing running, no watchers and
      no pull request anywhere.
- [ ] The branch holds no commits past the one it was cut from, and the Worktree
      is still there with its scratch in it.
