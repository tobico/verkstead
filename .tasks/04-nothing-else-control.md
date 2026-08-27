# 04. The Nothing-else control on a follow-up's Sets

## What to build

The closing section of a Question Set — the one the postscript and the
set-level comment box already share — draws a **Nothing else** option when,
and only when, the Set was asked while its Conversation is in Follow-up.
Picking it is the human saying the follow-up is over.

- The mark rides the **submitted Response**, the way a Proposal's direction
  pick does: a field of the Response rather than an Answer, recorded
  server-side beside the stored Response.
- **The agent knows nothing about it.** The Response the waiting CLI is
  handed is identical with or without the mark — no field, no comment, no
  difference. The agent writes an ordinary postscript and reads an ordinary
  Response; the ending is entirely the system's.
- Drawn on a Follow-up Conversation's Sets and **nowhere else** — no other
  state's Sets, no other part of the page. Nothing in this task acts on the
  mark; task 05 reads it.

## Acceptance criteria

- [ ] A Set asked while the Conversation is in Follow-up draws the Nothing
      else option in its closing section; Sets asked from any other state do
      not, and existing stored Sets are unaffected.
- [ ] Submitting a Response with the option picked stores the mark beside
      the Response; the Response handed to the waiting agent is
      byte-identical to one submitted without it.
- [ ] The mark is readable back per Set, so a later rule can ask whether the
      latest settled Set of a Conversation carries it.
