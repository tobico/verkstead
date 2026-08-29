# 02. Asking and idling

## Goal

The two mechanisms every full-screen-TUI backend needs exist and are proven
against the stub agents the suite already stands where Claude goes:
store-and-nudge asking (with the `verkstead answers` fetch), the Guide
tailored by agent type, and screen-signature quiet beside byte-quiet.
Demonstrable: a stub session on a store-and-nudge backend asks, ends its
turn, is nudged when the Response lands and fetches it — or, having died
first, the answers fold into the next session's prompt; and a stub drawing a
TUI-shaped screen is judged idle by its signature while byte-quiet alone no
longer ends it.

## Decisions in force

All from [ADR-0011](../../adr/0011-agent-backends.md); what bears on this
stage:

- **Store-and-nudge rides the Deferred Ask machinery.** The Set is stored as
  a `--deferred` one is; one storage shape, one folding rule. What is new is
  the nudge — one line typed into the session's terminal, the channel Rescue
  already uses — and `verkstead answers`, which prints a stored Set's
  Response. A session gone before the nudge is the folding rule's case
  already.
- **Which channel a Set was asked on is the backend's fact, not the Set's.**
  The CLI asks the same way everywhere; the server knows the Conversation's
  session's agent type and treats the ask accordingly. The Timeline and the
  badges already know how to say a deferred-shaped ask.
- **The Guide is tailored at print time.** Verkstead sets the agent type in
  the sandbox environment; `verkstead guide` prints the asking instructions
  for that backend — blocking with the hold-the-ask advice, or
  store-and-nudge with end-your-turn. One Guide, no skill forks, nothing
  extra in the prompt.
- **Screen-signature quiet.** For TUI backends, idle is the backend's
  at-the-prompt signature read off the Screen Verkstead already holds — one
  constant per backend, kept where `EXHAUSTED` is kept and accepted to
  drift the same way. Byte-quiet does not count as idle on a TUI backend
  (silent mid-turn would read as idle); Claude stays on byte-quiet
  unchanged. Idling, Rescue and session-ending all read the one judgement.

## Proposed tasks (provisional)

1. **`verkstead answers`.** Fetch a stored Set's Response by id; refused
   while unanswered; the Guide's store-and-nudge text names it.
   - A stored, answered Set prints as Response YAML on stdout, parseable as
     the blocking ask's output is.
2. **The ask channel per agent type.** The server stores a non-blocking
   backend's ask as deferred-shaped, keeps the session's claim on it, and
   the blocking path is untouched for Claude.
   - A stub-backend ask returns at once with the stored id; a Claude ask
     blocks as today.
3. **The nudge.** On a Response landing for a store-and-nudge Set whose
   session still runs: one canned line through the terminal; the folding
   rule untouched for a session that has gone.
   - The stub session receives the line and fetches; a killed stub's answers
     appear in the next session's prompt.
4. **Guide tailoring.** The agent type into the sandbox environment; the
   Guide's asking section per backend.
   - `verkstead guide` inside a stub sandbox prints the channel that
     backend's type names.
5. **Screen-signature idle.** The per-backend signature constant, the Screen
   read, and the idle judgement switched per agent type; Rescue and
   session-ending read it.
   - A repainting stub is never byte-idle but is judged idle at its
     signature; removing the signature makes it read busy until the ordinary
     stop.

## Re-verify at start

- Stage 01 landed: the agent type reaches the sandbox and the argv mapping
  is per-backend.
- The Deferred Ask folding rules (`asked deferred`, folded-once bookkeeping)
  are where the grilling left them — this stage reuses them verbatim.
- Rescue's typing channel is still the way keystrokes reach a session, and
  the canned-line pattern is still in one place.
- How the Screen is held server-side (`crate::screen`) and whether its
  emulated rows are readable where the idle judgement runs — the signature
  read wants the drawn frame, not the byte stream.
- IDLE_AFTER's uses: which of the idle mark, Rescue and the enders read
  byte-quiet directly, so the switch covers all of them and no caller keeps
  a private byte rule.
