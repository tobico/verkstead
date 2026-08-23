# 07. Retrying a stall for a grilling

## What to build

The fourth way back: a Grilling Conversation whose session died, retried.

A fresh grilling session on the Brief. The dead session's interview is gone —
a session is a process and nothing survives it — so this starts where a grilling
starts. What it is *also* given is the log of what was already asked: the
Conversation's answered Question Sets rendered as a markdown digest, so the new
session does not open by asking again what the human already settled.

The digest carries each Set's questions, the options chosen, whatever the human
wrote in their own words, and the set-level comment, in the order they were
asked. It is spliced into the prompt under the Brief, in the style the retry
note already uses: the newest and least general thing said goes last.

**An orphaned open Set is archived first.** A grilling session that died with a
Question Set still open leaves Answers with no reader — the human could answer
into nothing. Retrying archives that Set as orphaned before it relaunches, using
the archive path that already exists for one.

A Conversation with nothing answered yet — a grilling that died before its first
Set came back — relaunches on the Brief with no digest section at all. A heading
over nothing would tell the session that something had been said.

## Acceptance criteria

- [ ] Retrying a stalled grilling starts a fresh grilling session that registers
      as the driver, and its prompt carries a digest of every answered Set in
      the order they were asked
- [ ] A Set the dead session left open is archived as orphaned before the
      relaunch, so no Answer is left with nobody to read it
- [ ] A Conversation with nothing answered relaunches with no digest section and
      nothing else added to the prompt
