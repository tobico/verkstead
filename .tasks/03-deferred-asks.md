# 03. Deferred asks

## What to build

The second ask kind. A **Deferred Ask** does not idle the session that sends it:
the Set lands on the Timeline as any other does, the CLI returns as soon as it
is stored, and the session carries on. Its Answers reach a later session of that
Conversation, folded into that session's prompt. Work blocks only on Questions
whose Answers affect work about to be done, and this is what makes that rule
complete.

Three parts, end to end:

- **The CLI** grows a flag that submits and returns. It prints enough for the
  agent to know the Set was stored and which one it is, on the same stdout
  contract the blocking ask keeps — nothing else has ever been written there.
  Verkstead may break the protocol freely; there is no wire-compatibility
  obligation to askance.
- **The Timeline and the sidebar** tell an unanswered deferred Set from an
  unanswered blocking one. Both are something to answer, so both keep the
  sidebar's *blocked on you* and both push to the human's devices exactly as a
  blocking Set does — what differs is that no session is idling on this one, and
  the Timeline says which kind it is.
- **The folding.** When a session is started for a Conversation, every deferred
  Set that has been answered and not yet folded goes into its prompt, oldest
  first, under the documents the prompt is built from — where a retry note goes,
  and for its reason: it is newer and less general than the Brief. Each is
  folded once and never again, which means the folding is recorded rather than
  recomputed from what is answered.

The Guide the CLI ships says which ask to use when, in the terms the design
uses: block only on what the next stretch of work turns on.

## Acceptance criteria

- [ ] A deferred ask returns as soon as its Set is stored, saying which Set it
      is, and the session goes on working; the Set is on the Timeline and
      answerable from the workbench and the phone alike.
- [ ] The Timeline distinguishes an unanswered deferred Set from an unanswered
      blocking one, and both keep the Conversation *blocked on you* and both
      notify.
- [ ] The Answers to an answered deferred Set appear in the next session started
      on that Conversation, oldest first, and appear in no session after that.
- [ ] The Guide names the two kinds and says which to use when.
