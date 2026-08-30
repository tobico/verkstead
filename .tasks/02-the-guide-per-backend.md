# 02. The agent type reaches the sandbox, and the Guide is tailored to it

## What to build

A session learns which channel is its own from the Guide, and the Guide learns
which backend it is being printed for. Which means two things land together: the
agent type has to be inside the sandbox for `verkstead guide` to read, and there
has to be a second agent type for any of it to be worth reading — so `codex`
becomes a word the store knows here, ahead of the stage that makes it launch
the real thing.

**The type in the sandbox environment.** Verkstead sets it as it starts a
session, off the Pairing's Profile, beside the server address and the terminal
kind it already sets. Nothing else inside a sandbox needs it — the ask channel
and the idle judgement are both decided server-side — so this exists for the
Guide alone, and a Guide printed outside a sandbox, with nothing set, is the
blocking one.

**The Guide's asking sections come in one per channel.** The Guide is one
document embedded in the binary and it stays one document; what varies is the
part that describes how an ask is run and what comes back:

- **Blocking** — as today, including the advice about holding the ask open in a
  background shell call, which is Claude Code's mechanism and is true of it.
- **Store-and-nudge** — the ask returns as soon as the Set is stored, so end
  your turn; Verkstead types a line into this terminal when the Response lands,
  and `verkstead answers <id>` is how the Answers are fetched then.

`--deferred` keeps meaning what it means on both: an ask nobody is idling on,
whose Answers reach a later session. Everything about writing a Set is common
and is not duplicated.

**`codex` becomes a word `AgentType` knows.** The variant, and the account
shape ADR-0011 gives every backend after Claude: one relocatable home, kept in
the table stage 01 made for it, bound where that backend expects it —
`~/.codex`. Reading a Profile back, saving one, and taking one away all carry
the home the way they carry the pair. Claude's pair is untouched and every saved
Profile still reads back unchanged.

**What stays stage 03's.** The launch line — the bypass flags, the trust
pre-seed, the model and prompt positions — the rollout log discovery and its
renderer, the usage-limit phrase, the idle signature constant, and the form row
offering the type to the human. A type that cannot launch the real binary is
still not offered in a picker. What this task owes is that everything a session
touches has an arm for the new type rather than a shape it assumes: the sandbox
binds the home, the Transcript reader finds no log and the Capture is the record
(ADR-0006's rule, unchanged), and the broken-Profile rules read the home the way
they read the pair.

**Which is what gives the suite its second backend.** A stub program standing
where the agent goes, launched under a Profile of the new type, is what tasks 03
to 05 are proven against.

## Acceptance criteria

- [ ] `verkstead guide` inside a sandbox on a store-and-nudge backend prints
      that channel and names `verkstead answers`; inside a Claude one it prints
      today's blocking text unchanged; run with nothing set it prints the
      blocking one.
- [ ] A Profile of the new type round-trips through the store with its home,
      launches a stub session into a sandbox with that home bound where the
      backend expects it, and leaves the Capture as its record with no
      Transcript.
- [ ] Every Claude Profile already saved reads back unchanged, the form still
      offers Claude alone, and a Claude session behaves exactly as it does
      today.
